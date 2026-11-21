// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Streaming subscriber session management, lease configuration, and settlement handling.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use google_cloud_auth::credentials::anonymous::Builder as AnonymousCredentials;
use google_cloud_gax::error::rpc::StatusDetails;
use google_cloud_pubsub::client::Subscriber;
use google_cloud_pubsub::error::AckError;
use google_cloud_pubsub::model::Message;
use google_cloud_pubsub::subscriber::handler::Handler;
use google_cloud_pubsub::subscriber::{MessageStream, ShutdownBehavior as SdkShutdownBehavior};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tonic::{Response, Status, Streaming};

use crate::pubsub::conformance::v1::settle_message_command::SettleAction;
use crate::pubsub::conformance::v1::settlement_result_event::AckStatus;
use crate::pubsub::conformance::v1::start_subscriber_command::AutoSettleMode;
use crate::pubsub::conformance::v1::subscriber_command::Command;
use crate::pubsub::conformance::v1::subscriber_event::Event;
use crate::pubsub::conformance::v1::{
    MessageDeliveredEvent, SettleMessageCommand, SettlementResultEvent, ShutdownBehavior,
    StartSubscriberCommand, StartedEvent, StopSubscriberCommand, StoppedEvent, SubscriberCommand,
    SubscriberEvent,
};
use crate::server::{
    ShutdownController, check_common_overrides, map_gax_error, resolve_endpoint,
    resolve_resource_path, wait_for_watch_true,
};

type NextMsgFut = Pin<
    Box<
        dyn Future<
                Output = (
                    MessageStream,
                    Option<google_cloud_pubsub::Result<(Message, Handler)>>,
                ),
            > + Send,
    >,
>;
type ShutdownFut = Pin<Box<dyn Future<Output = bool> + Send>>;

/// Server-streaming response type for `PubSubProxy::subscriber_session`.
#[derive(Debug)]
pub struct SubscriberSessionStream {
    rx: UnboundedReceiver<Result<SubscriberEvent, Status>>,
}

impl tonic::codegen::tokio_stream::Stream for SubscriberSessionStream {
    type Item = Result<SubscriberEvent, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

fn send_event(tx: &UnboundedSender<Result<SubscriberEvent, Status>>, event: Event) -> bool {
    tx.send(Ok(SubscriberEvent { event: Some(event) })).is_ok()
}

/// Validates `StartSubscriberCommand` and launches the background subscriber session loop.
pub async fn start_subscriber_session(
    mut in_stream: Streaming<SubscriberCommand>,
    shutdown_ctrl: ShutdownController,
) -> Result<Response<SubscriberSessionStream>, Status> {
    if shutdown_ctrl.is_shutting_down() {
        return Err(Status::unavailable("proxy is shutting down"));
    }

    let first = in_stream
        .message()
        .await
        .map_err(|e| Status::invalid_argument(format!("failed reading start command: {e}")))?
        .ok_or_else(|| Status::invalid_argument("subscriber session ended before Start command"))?;

    let start = match first.command {
        Some(Command::Start(start)) => start,
        _ => {
            return Err(Status::invalid_argument(
                "first command in SubscriberSession must be StartSubscriberCommand",
            ));
        }
    };

    validate_start_command(&start)?;
    let endpoint = resolve_endpoint(&start.endpoint, start.use_emulator_env)?;
    let subscription =
        resolve_resource_path("subscriptions", &start.subscription, &start.project_id)?;

    let client = Subscriber::builder()
        .with_endpoint(endpoint)
        .with_credentials(AnonymousCredentials::new().build())
        .build()
        .await
        .map_err(|e| Status::internal(format!("failed to build subscriber: {e}")))?;

    if shutdown_ctrl.is_shutting_down() {
        return Err(Status::unavailable("proxy is shutting down"));
    }

    let mut subscribe = client.subscribe(subscription);
    if let Some(v) = start.max_outstanding_messages {
        subscribe = subscribe.set_max_outstanding_messages(v);
    }
    if let Some(v) = start.max_outstanding_bytes {
        subscribe = subscribe.set_max_outstanding_bytes(v);
    }
    if let Some(ms) = start.max_extension_ms {
        let dur = if ms < 0 {
            Duration::MAX
        } else {
            Duration::from_millis(ms as u64)
        };
        subscribe = subscribe.set_max_lease(dur);
    }
    if let Some(ms) = start.max_duration_per_ack_extension_ms {
        let dur = Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0));
        subscribe = subscribe.set_max_lease_extension(dur);
    }

    let built_shutdown_behavior = match ShutdownBehavior::try_from(start.shutdown_behavior)
        .unwrap_or(ShutdownBehavior::Unspecified)
    {
        ShutdownBehavior::NackImmediately => {
            subscribe = subscribe.set_shutdown_behavior(SdkShutdownBehavior::NackImmediately);
            ShutdownBehavior::NackImmediately
        }
        ShutdownBehavior::WaitForProcessing | ShutdownBehavior::Unspecified => {
            ShutdownBehavior::WaitForProcessing
        }
    };

    let auto_settle_mode =
        AutoSettleMode::try_from(start.auto_settle_mode).unwrap_or(AutoSettleMode::Unspecified);

    let stream = subscribe.build();
    let (tx, rx) = unbounded_channel();

    tokio::spawn(run_subscriber_session(
        stream,
        in_stream,
        tx,
        built_shutdown_behavior,
        auto_settle_mode,
        shutdown_ctrl,
    ));

    Ok(Response::new(SubscriberSessionStream { rx }))
}

fn validate_start_command(start: &StartSubscriberCommand) -> Result<(), Status> {
    check_common_overrides(&start.client_id, &start.custom_headers)?;

    if start.enable_per_stream_flow_control.is_some() {
        return Err(Status::unimplemented(
            "enable_per_stream_flow_control is not supported by google-cloud-pubsub",
        ));
    }
    if start.num_streams.is_some_and(|n| n != 1) {
        return Err(Status::unimplemented(
            "num_streams != 1 is not supported by google-cloud-pubsub",
        ));
    }
    if start.max_concurrency.is_some() {
        return Err(Status::unimplemented(
            "max_concurrency is not supported by google-cloud-pubsub",
        ));
    }
    if start.shutdown_timeout_ms.is_some() {
        return Err(Status::unimplemented(
            "shutdown_timeout_ms is not supported by google-cloud-pubsub",
        ));
    }
    if !start.retry_codes.is_empty()
        || start.initial_retry_delay_ms.is_some()
        || start.max_retry_delay_ms.is_some()
        || start.retry_delay_multiplier.is_some()
    {
        return Err(Status::unimplemented(
            "custom subscriber retry configuration is not supported by google-cloud-pubsub",
        ));
    }
    if start.shutdown_polling_period_ms.is_some() {
        return Err(Status::unimplemented(
            "shutdown_polling_period_ms is not supported by google-cloud-pubsub",
        ));
    }
    // Note: `enable_monitoring`, `use_exactly_once`, and
    // `min_duration_per_ack_extension_ms` are intentionally accepted cleanly
    // (TRAC-03 / LEASE-02 / EOD-01).
    let _ = (
        start.enable_monitoring,
        start.use_exactly_once,
        start.min_duration_per_ack_extension_ms,
    );
    Ok(())
}

fn arm_shutdown(
    shutdown_fut: &mut Option<ShutdownFut>,
    shutdown_completed: bool,
    shutdown_token: &google_cloud_pubsub::subscriber::ShutdownToken,
    next_msg_fut: &mut Option<NextMsgFut>,
    msg_stream_slot: &mut Option<MessageStream>,
    max_wait: Option<Duration>,
) {
    // Stop pulling new messages and close the underlying StreamingPull RPC.
    *next_msg_fut = None;
    *msg_stream_slot = None;
    if shutdown_fut.is_none() && !shutdown_completed {
        let tok = shutdown_token.clone();
        *shutdown_fut = Some(Box::pin(async move {
            if let Some(dur) = max_wait {
                tokio::time::timeout(dur, tok.shutdown()).await.is_err()
            } else {
                tok.shutdown().await;
                false
            }
        }));
    }
}

async fn run_subscriber_session(
    msg_stream: MessageStream,
    mut in_stream: Streaming<SubscriberCommand>,
    tx: UnboundedSender<Result<SubscriberEvent, Status>>,
    built_shutdown_behavior: ShutdownBehavior,
    auto_settle_mode: AutoSettleMode,
    shutdown_ctrl: ShutdownController,
) {
    let mut global_shutdown_rx: watch::Receiver<bool> = shutdown_ctrl.subscribe();
    let shutdown_token = msg_stream.shutdown_token();

    if *global_shutdown_rx.borrow_and_update() {
        drop(msg_stream);
        let grace = shutdown_ctrl.grace_period();
        let _ = tokio::time::timeout(grace, shutdown_token.shutdown()).await;
        let _ = send_event(
            &tx,
            Event::Stopped(StoppedEvent {
                error_code: 14,
                error_message: "proxy is shutting down".to_string(),
                total_delivered: 0,
            }),
        );
        return;
    }

    if !send_event(
        &tx,
        Event::Started(StartedEvent {
            client_id: String::new(),
        }),
    ) {
        drop(msg_stream);
        let _ = tokio::time::timeout(Duration::from_millis(500), shutdown_token.shutdown()).await;
        return;
    }

    let is_auto_settle = !matches!(
        auto_settle_mode,
        AutoSettleMode::Unspecified | AutoSettleMode::Interactive
    );
    let mut msg_stream_slot: Option<MessageStream> = Some(msg_stream);
    let mut next_msg_fut: Option<NextMsgFut> = None;
    let mut in_flight: HashMap<String, Handler> = HashMap::new();
    let active_callbacks = Arc::new(AtomicI32::new(0));
    let mut settle_tasks: JoinSet<SettlementResultEvent> = JoinSet::new();

    let mut total_delivered: i64 = 0;
    let mut stream_err: Option<(i32, String)> = None;
    let mut terminal_status: Option<Status> = None;
    let mut msg_stream_done = false;
    let mut in_stream_closed = false;
    let mut stop_pending_wait = false;
    let mut shutdown_completed = false;
    let mut shutdown_timed_out = false;
    let mut shutdown_fut: Option<ShutdownFut> = None;
    let mut global_shutdown_handled = false;
    let mut tx_closed = false;

    loop {
        if shutdown_completed && settle_tasks.is_empty() {
            break;
        }
        if global_shutdown_handled && shutdown_completed && shutdown_timed_out {
            settle_tasks.abort_all();
            break;
        }

        if next_msg_fut.is_none()
            && !msg_stream_done
            && !stop_pending_wait
            && shutdown_fut.is_none()
            && !shutdown_completed
            && let Some(mut stream) = msg_stream_slot.take()
        {
            next_msg_fut = Some(Box::pin(async move {
                let item = stream.next().await;
                (stream, item)
            }));
        }

        tokio::select! {
            biased;

            task_res = settle_tasks.join_next(), if !settle_tasks.is_empty() => {
                if let Some(Ok(ev)) = task_res {
                    let _ = send_event(&tx, Event::SettlementResult(ev));
                }
                if stop_pending_wait && in_flight.is_empty() && settle_tasks.is_empty() {
                    stop_pending_wait = false;
                    arm_shutdown(
                        &mut shutdown_fut,
                        shutdown_completed,
                        &shutdown_token,
                        &mut next_msg_fut,
                        &mut msg_stream_slot,
                        None,
                    );
                }
            }

            timed_out = async {
                if let Some(fut) = shutdown_fut.as_mut() {
                    fut.as_mut().await
                } else {
                    false
                }
            }, if shutdown_fut.is_some() => {
                shutdown_fut = None;
                shutdown_completed = true;
                shutdown_timed_out = timed_out;
            }

            _ = wait_for_watch_true(&mut global_shutdown_rx), if !global_shutdown_handled => {
                global_shutdown_handled = true;
                in_stream_closed = true;
                stop_pending_wait = false;
                in_flight.clear();
                active_callbacks.store(0, Ordering::Release);
                let grace = shutdown_ctrl.grace_period();
                if let Some(existing) = shutdown_fut.take() {
                    next_msg_fut = None;
                    msg_stream_slot = None;
                    shutdown_fut = Some(Box::pin(async move {
                        tokio::time::timeout(grace, existing).await.unwrap_or(true)
                    }));
                } else {
                    arm_shutdown(
                        &mut shutdown_fut,
                        shutdown_completed,
                        &shutdown_token,
                        &mut next_msg_fut,
                        &mut msg_stream_slot,
                        Some(grace),
                    );
                }
            }

            _ = tx.closed(), if !tx_closed => {
                tx_closed = true;
                in_stream_closed = true;
                stop_pending_wait = false;
                in_flight.clear();
                active_callbacks.store(0, Ordering::Release);
                arm_shutdown(
                    &mut shutdown_fut,
                    shutdown_completed,
                    &shutdown_token,
                    &mut next_msg_fut,
                    &mut msg_stream_slot,
                    Some(Duration::from_millis(500)),
                );
            }

            cmd_res = in_stream.message(), if !in_stream_closed => {
                match cmd_res {
                    Ok(Some(cmd)) => match cmd.command {
                        Some(Command::Settle(settle_cmd)) => {
                            handle_settle_command(
                                settle_cmd,
                                &mut in_flight,
                                &active_callbacks,
                                &mut settle_tasks,
                            );
                            if stop_pending_wait && in_flight.is_empty() && settle_tasks.is_empty() {
                                stop_pending_wait = false;
                                arm_shutdown(
                                    &mut shutdown_fut,
                                    shutdown_completed,
                                    &shutdown_token,
                                    &mut next_msg_fut,
                                    &mut msg_stream_slot,
                                    None,
                                );
                            }
                        }
                        Some(Command::Stop(stop_cmd)) => {
                            if stop_cmd.shutdown_timeout_ms.is_some() {
                                in_flight.clear();
                                active_callbacks.store(0, Ordering::Release);
                                stop_pending_wait = false;
                                terminal_status = Some(Status::unimplemented(
                                    "shutdown_timeout_ms is not supported by google-cloud-pubsub",
                                ));
                                arm_shutdown(
                                    &mut shutdown_fut,
                                    shutdown_completed,
                                    &shutdown_token,
                                    &mut next_msg_fut,
                                    &mut msg_stream_slot,
                                    Some(Duration::from_millis(500)),
                                );
                            } else {
                                handle_stop_command(
                                    &stop_cmd,
                                    built_shutdown_behavior,
                                    &mut in_flight,
                                    &active_callbacks,
                                    &settle_tasks,
                                    &shutdown_token,
                                    &mut next_msg_fut,
                                    &mut msg_stream_slot,
                                    &mut stop_pending_wait,
                                    &mut shutdown_fut,
                                    shutdown_completed,
                                );
                            }
                        }
                        Some(Command::Start(_)) => {
                            in_flight.clear();
                            active_callbacks.store(0, Ordering::Release);
                            stop_pending_wait = false;
                            terminal_status = Some(Status::failed_precondition(
                                "StartSubscriberCommand already received in this session",
                            ));
                            arm_shutdown(
                                &mut shutdown_fut,
                                shutdown_completed,
                                &shutdown_token,
                                &mut next_msg_fut,
                                &mut msg_stream_slot,
                                Some(Duration::from_millis(500)),
                            );
                        }
                        None => {}
                    },
                    Ok(None) => {
                        in_stream_closed = true;
                        if !is_auto_settle {
                            in_flight.clear();
                            active_callbacks.store(0, Ordering::Release);
                            stop_pending_wait = false;
                            arm_shutdown(
                                &mut shutdown_fut,
                                shutdown_completed,
                                &shutdown_token,
                                &mut next_msg_fut,
                                &mut msg_stream_slot,
                                None,
                            );
                        } else if !in_flight.is_empty() {
                            in_flight.clear();
                            active_callbacks.store(0, Ordering::Release);
                        }
                    }
                    Err(_) => {
                        in_stream_closed = true;
                        in_flight.clear();
                        active_callbacks.store(0, Ordering::Release);
                        stop_pending_wait = false;
                        arm_shutdown(
                            &mut shutdown_fut,
                            shutdown_completed,
                            &shutdown_token,
                            &mut next_msg_fut,
                            &mut msg_stream_slot,
                            None,
                        );
                    }
                }
            }

            (returned_stream, next_msg) = async {
                next_msg_fut
                    .as_mut()
                    .expect("next_msg_fut populated before select")
                    .as_mut()
                    .await
            }, if next_msg_fut.is_some() && !msg_stream_done && !stop_pending_wait && shutdown_fut.is_none() && !shutdown_completed => {
                next_msg_fut = None;
                msg_stream_slot = Some(returned_stream);
                match next_msg {
                    Some(Ok((msg, handler))) => {
                        total_delivered += 1;
                        let token = format!("del-{total_delivered}");
                        let (delivery_attempt, has_delivery_attempt) = match handler.delivery_attempt() {
                            Some(n) => (n, true),
                            None => (0, false),
                        };
                        let is_exactly_once = matches!(handler, Handler::ExactlyOnce(_));
                        let auto_settled = is_auto_settle;
                        let active = active_callbacks.fetch_add(1, Ordering::AcqRel) + 1;
                        let publish_time = msg.publish_time.map(|ts| prost_types::Timestamp {
                            seconds: ts.seconds(),
                            nanos: ts.nanos(),
                        });

                        let delivered_event = Event::MessageDelivered(MessageDeliveredEvent {
                            delivery_token: token.clone(),
                            message_id: msg.message_id,
                            data: msg.data.to_vec(),
                            attributes: msg.attributes,
                            ordering_key: msg.ordering_key,
                            publish_time,
                            delivery_attempt,
                            has_delivery_attempt,
                            active_callbacks: active,
                            callback_traceparent: String::new(),
                            callback_tracestate: String::new(),
                            is_exactly_once,
                            auto_settled,
                        });

                        if auto_settled {
                            if !send_event(&tx, delivered_event) {
                                drop(handler);
                                decrement_active(&active_callbacks);
                                tx_closed = true;
                                in_stream_closed = true;
                                stop_pending_wait = false;
                                in_flight.clear();
                                active_callbacks.store(0, Ordering::Release);
                                arm_shutdown(
                                    &mut shutdown_fut,
                                    shutdown_completed,
                                    &shutdown_token,
                                    &mut next_msg_fut,
                                    &mut msg_stream_slot,
                                    Some(Duration::from_millis(500)),
                                );
                            } else {
                                apply_auto_settle(
                                    auto_settle_mode,
                                    token,
                                    handler,
                                    &active_callbacks,
                                    &mut settle_tasks,
                                );
                            }
                        } else {
                            in_flight.insert(token.clone(), handler);
                            if !send_event(&tx, delivered_event) {
                                if in_flight.remove(&token).is_some() {
                                    decrement_active(&active_callbacks);
                                }
                                tx_closed = true;
                                in_stream_closed = true;
                                stop_pending_wait = false;
                                in_flight.clear();
                                active_callbacks.store(0, Ordering::Release);
                                arm_shutdown(
                                    &mut shutdown_fut,
                                    shutdown_completed,
                                    &shutdown_token,
                                    &mut next_msg_fut,
                                    &mut msg_stream_slot,
                                    Some(Duration::from_millis(500)),
                                );
                            }
                        }
                    }
                    Some(Err(err)) => {
                        msg_stream_done = true;
                        stop_pending_wait = false;
                        if stream_err.is_none() {
                            stream_err = Some(map_gax_error(&err));
                        }
                        arm_shutdown(
                            &mut shutdown_fut,
                            shutdown_completed,
                            &shutdown_token,
                            &mut next_msg_fut,
                            &mut msg_stream_slot,
                            None,
                        );
                    }
                    None => {
                        msg_stream_done = true;
                        stop_pending_wait = false;
                        arm_shutdown(
                            &mut shutdown_fut,
                            shutdown_completed,
                            &shutdown_token,
                            &mut next_msg_fut,
                            &mut msg_stream_slot,
                            None,
                        );
                    }
                }
            }
        }
    }

    if let Some(status) = terminal_status {
        let _ = tx.send(Err(status));
    } else if !tx_closed {
        let (error_code, error_message) = stream_err.unwrap_or_default();
        let _ = send_event(
            &tx,
            Event::Stopped(StoppedEvent {
                error_code,
                error_message,
                total_delivered,
            }),
        );
    }
}

fn decrement_active(active_callbacks: &AtomicI32) {
    let mut cur = active_callbacks.load(Ordering::Acquire);
    while cur > 0 {
        match active_callbacks.compare_exchange_weak(
            cur,
            cur - 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(next) => cur = next,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_stop_command(
    stop_cmd: &StopSubscriberCommand,
    built_shutdown_behavior: ShutdownBehavior,
    in_flight: &mut HashMap<String, Handler>,
    active_callbacks: &AtomicI32,
    settle_tasks: &JoinSet<SettlementResultEvent>,
    shutdown_token: &google_cloud_pubsub::subscriber::ShutdownToken,
    next_msg_fut: &mut Option<NextMsgFut>,
    msg_stream_slot: &mut Option<MessageStream>,
    stop_pending_wait: &mut bool,
    shutdown_fut: &mut Option<ShutdownFut>,
    shutdown_completed: bool,
) {
    let override_behavior = ShutdownBehavior::try_from(stop_cmd.shutdown_behavior)
        .unwrap_or(ShutdownBehavior::Unspecified);
    let effective = match override_behavior {
        ShutdownBehavior::Unspecified => built_shutdown_behavior,
        other => other,
    };

    // SUBF-01 deadlock guard: if effective shutdown is NACK_IMMEDIATELY, drop any
    // held in-flight handlers immediately so auto-nack fires and WaitForProcessing
    // cannot hang awaiting interactive Settle commands that will never arrive.
    if effective == ShutdownBehavior::NackImmediately {
        in_flight.clear();
        active_callbacks.store(0, Ordering::Release);
        *stop_pending_wait = false;
        arm_shutdown(
            shutdown_fut,
            shutdown_completed,
            shutdown_token,
            next_msg_fut,
            msg_stream_slot,
            None,
        );
        return;
    }

    // SUB_PROBE_3: if the stream was built with NackImmediately, cancelling or
    // dropping `MessageStream` before `in_flight` / `settle_tasks` finish causes
    // `LeaseLoop` to nack immediately; pause message delivery and defer shutdown
    // until all active handlers and settlements drain.
    if built_shutdown_behavior == ShutdownBehavior::NackImmediately
        && (!in_flight.is_empty() || !settle_tasks.is_empty())
    {
        *stop_pending_wait = true;
        return;
    }

    *stop_pending_wait = false;
    arm_shutdown(
        shutdown_fut,
        shutdown_completed,
        shutdown_token,
        next_msg_fut,
        msg_stream_slot,
        None,
    );
}

fn handle_settle_command(
    cmd: SettleMessageCommand,
    in_flight: &mut HashMap<String, Handler>,
    active_callbacks: &Arc<AtomicI32>,
    settle_tasks: &mut JoinSet<SettlementResultEvent>,
) {
    let Some(handler) = in_flight.remove(&cmd.delivery_token) else {
        return;
    };
    let action = SettleAction::try_from(cmd.action).unwrap_or(SettleAction::Unspecified);
    match action {
        SettleAction::Ack | SettleAction::Unspecified => {
            handler.ack();
            decrement_active(active_callbacks);
        }
        SettleAction::Nack => {
            handler.nack();
            decrement_active(active_callbacks);
        }
        SettleAction::ReturnWithoutSettle => {
            drop(handler);
            decrement_active(active_callbacks);
        }
        SettleAction::AckWithResult => {
            spawn_confirmed_settle(
                cmd.delivery_token,
                handler,
                false,
                Arc::clone(active_callbacks),
                settle_tasks,
            );
        }
        SettleAction::NackWithResult => {
            spawn_confirmed_settle(
                cmd.delivery_token,
                handler,
                true,
                Arc::clone(active_callbacks),
                settle_tasks,
            );
        }
    }
}

fn apply_auto_settle(
    mode: AutoSettleMode,
    token: String,
    handler: Handler,
    active_callbacks: &Arc<AtomicI32>,
    settle_tasks: &mut JoinSet<SettlementResultEvent>,
) {
    match mode {
        AutoSettleMode::AutoAck => {
            handler.ack();
            decrement_active(active_callbacks);
        }
        AutoSettleMode::AutoNack => {
            handler.nack();
            decrement_active(active_callbacks);
        }
        AutoSettleMode::AutoDropWithoutSettle => {
            drop(handler);
            decrement_active(active_callbacks);
        }
        AutoSettleMode::AutoAckWithResult => {
            spawn_confirmed_settle(
                token,
                handler,
                false,
                Arc::clone(active_callbacks),
                settle_tasks,
            );
        }
        AutoSettleMode::AutoNackWithResult => {
            spawn_confirmed_settle(
                token,
                handler,
                true,
                Arc::clone(active_callbacks),
                settle_tasks,
            );
        }
        AutoSettleMode::Unspecified | AutoSettleMode::Interactive => {
            drop(handler);
            decrement_active(active_callbacks);
        }
    }
}

fn spawn_confirmed_settle(
    delivery_token: String,
    handler: Handler,
    is_nack: bool,
    active_callbacks: Arc<AtomicI32>,
    settle_tasks: &mut JoinSet<SettlementResultEvent>,
) {
    settle_tasks.spawn(async move {
        let ev = settle_with_result(delivery_token, handler, is_nack).await;
        decrement_active(&active_callbacks);
        ev
    });
}

/// Settles `handler` with confirmation, synthesizing `AckStatus::Success` for `AtLeastOnce` (EOD-02).
pub async fn settle_with_result(
    delivery_token: String,
    handler: Handler,
    is_nack: bool,
) -> SettlementResultEvent {
    match handler {
        Handler::AtLeastOnce(h) => {
            if is_nack {
                h.nack();
            } else {
                h.ack();
            }
            SettlementResultEvent {
                delivery_token,
                status: AckStatus::Success as i32,
                grpc_error_code: 0,
                error_message: String::new(),
                is_nack,
            }
        }
        Handler::ExactlyOnce(h) => {
            let res = if is_nack {
                h.confirmed_nack().await
            } else {
                h.confirmed_ack().await
            };
            match res {
                Ok(()) => SettlementResultEvent {
                    delivery_token,
                    status: AckStatus::Success as i32,
                    grpc_error_code: 0,
                    error_message: String::new(),
                    is_nack,
                },
                Err(err) => {
                    let (status, grpc_error_code, error_message) = map_ack_error(&err);
                    SettlementResultEvent {
                        delivery_token,
                        status,
                        grpc_error_code,
                        error_message,
                        is_nack,
                    }
                }
            }
        }
        other => {
            if is_nack {
                other.nack();
            } else {
                other.ack();
            }
            SettlementResultEvent {
                delivery_token,
                status: AckStatus::Other as i32,
                grpc_error_code: 2,
                error_message: "unsupported handler variant".to_string(),
                is_nack,
            }
        }
    }
}

/// Translates a [`google_cloud_gax::error::Error`] from an EOD ack/nack RPC into
/// `(AckStatus as i32, grpc_error_code, error_message)`.
pub fn classify_ack_gax_error(source: &google_cloud_gax::error::Error) -> (i32, i32, String) {
    let (grpc_code, msg) = map_gax_error(source);
    let has_invalid_ack_info = source.status().is_some_and(|st| {
        st.details.iter().any(|detail| match detail {
            StatusDetails::ErrorInfo(info) => info
                .metadata
                .values()
                .any(|v| v.contains("PERMANENT_FAILURE_INVALID_ACK_ID")),
            _ => false,
        })
    });
    let status = if has_invalid_ack_info || msg.contains("PERMANENT_FAILURE_INVALID_ACK_ID") {
        AckStatus::InvalidAckId
    } else {
        match grpc_code {
            7 => AckStatus::PermissionDenied,
            9 => AckStatus::FailedPrecondition,
            14 => AckStatus::Unavailable,
            2 => AckStatus::Unknown,
            _ => AckStatus::Other,
        }
    };
    (status as i32, grpc_code, msg)
}

/// Translates an [`AckError`] into `(AckStatus as i32, grpc_error_code, error_message)`.
pub fn map_ack_error(err: &AckError) -> (i32, i32, String) {
    match err {
        AckError::LeaseExpired => (AckStatus::InvalidAckId as i32, 0, err.to_string()),
        AckError::Rpc { source, .. } => classify_ack_gax_error(source.as_ref()),
        AckError::ShutdownBeforeAck | AckError::Shutdown(_) => {
            (AckStatus::Other as i32, 2, err.to_string())
        }
        _ => (AckStatus::Other as i32, 2, err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_cloud_gax::error::Error as GaxError;
    use google_cloud_gax::error::rpc::{Code as GaxCode, Status as RpcStatus};

    #[test]
    fn ack_error_maps_eod_statuses() {
        assert_eq!(
            map_ack_error(&AckError::LeaseExpired).0,
            AckStatus::InvalidAckId as i32
        );
        assert_eq!(
            map_ack_error(&AckError::ShutdownBeforeAck),
            (
                AckStatus::Other as i32,
                2,
                AckError::ShutdownBeforeAck.to_string()
            )
        );
        let shutdown_err = AckError::Shutdown(Box::new(std::io::Error::other("loop closed")));
        assert_eq!(map_ack_error(&shutdown_err).0, AckStatus::Other as i32);

        let perm_denied = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::PermissionDenied)
                .set_message("denied"),
        );
        assert_eq!(
            classify_ack_gax_error(&perm_denied),
            (AckStatus::PermissionDenied as i32, 7, "denied".to_string())
        );

        let failed_precond = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::FailedPrecondition)
                .set_message("precond"),
        );
        assert_eq!(
            classify_ack_gax_error(&failed_precond).0,
            AckStatus::FailedPrecondition as i32
        );

        let invalid_ack = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::InvalidArgument)
                .set_message("PERMANENT_FAILURE_INVALID_ACK_ID"),
        );
        assert_eq!(
            classify_ack_gax_error(&invalid_ack),
            (
                AckStatus::InvalidAckId as i32,
                3,
                "PERMANENT_FAILURE_INVALID_ACK_ID".to_string()
            )
        );

        let raw_error_info = prost::bytes::Bytes::from_static(
            br#"{"error":{"code":400,"status":"INVALID_ARGUMENT","message":"ack failed","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACK_FAILED","metadata":{"ack-1":"PERMANENT_FAILURE_INVALID_ACK_ID"}}]}}"#,
        );
        let error_info_status = RpcStatus::try_from(&raw_error_info).expect("valid error json");
        let error_info_gax = GaxError::service(error_info_status);
        assert_eq!(
            classify_ack_gax_error(&error_info_gax),
            (AckStatus::InvalidAckId as i32, 3, "ack failed".to_string())
        );

        let unavail = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::Unavailable)
                .set_message("try again"),
        );
        assert_eq!(
            classify_ack_gax_error(&unavail),
            (AckStatus::Unavailable as i32, 14, "try again".to_string())
        );

        let unknown = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::Unknown)
                .set_message("unknown error"),
        );
        assert_eq!(
            classify_ack_gax_error(&unknown),
            (AckStatus::Unknown as i32, 2, "unknown error".to_string())
        );

        let timeout = GaxError::timeout("deadline");
        assert_eq!(classify_ack_gax_error(&timeout).0, AckStatus::Other as i32);
        assert_eq!(classify_ack_gax_error(&timeout).1, 4);

        let other_invalid = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::InvalidArgument)
                .set_message("bad field"),
        );
        assert_eq!(
            classify_ack_gax_error(&other_invalid),
            (AckStatus::Other as i32, 3, "bad field".to_string())
        );
    }

    #[test]
    fn start_validation_accepts_min_ack_ext_and_single_stream_rejects_others() {
        let ok_cmd = StartSubscriberCommand {
            endpoint: "127.0.0.1:1".to_string(),
            subscription: "projects/p/subscriptions/s".to_string(),
            min_duration_per_ack_extension_ms: Some(10_000),
            num_streams: Some(1),
            enable_monitoring: true,
            ..Default::default()
        };
        assert!(validate_start_command(&ok_cmd).is_ok());

        let mut headers = HashMap::new();
        headers.insert("x-k".to_string(), "v".to_string());

        for bad in [
            StartSubscriberCommand {
                num_streams: Some(2),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                enable_per_stream_flow_control: Some(false),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                max_concurrency: Some(4),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                shutdown_timeout_ms: Some(1000),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                shutdown_polling_period_ms: Some(100),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                retry_codes: vec![14],
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                initial_retry_delay_ms: Some(100),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                max_retry_delay_ms: Some(1000),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                retry_delay_multiplier: Some(2.0),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                client_id: "cid".to_string(),
                ..ok_cmd.clone()
            },
            StartSubscriberCommand {
                custom_headers: headers,
                ..ok_cmd
            },
        ] {
            let err = validate_start_command(&bad).unwrap_err();
            assert_eq!(err.code(), Status::unimplemented("").code());
        }
    }
}
