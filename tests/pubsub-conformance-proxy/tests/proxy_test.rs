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

//! End-to-end gRPC and binary subprocess integration tests for `pubsub-conformance-proxy`.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pubsub_conformance_proxy::pubsub::conformance::v1::pub_sub_proxy_client::PubSubProxyClient;
use pubsub_conformance_proxy::pubsub::conformance::v1::{
    AwaitPublishRequest, ClosePublisherRequest, CreatePublisherRequest, DrainTracesRequest,
    FlushPublisherRequest, GetCapabilitiesRequest, HedgingSettings, PublishRequest, RawAttribute,
    ResetTracesRequest, ResumePublishRequest, SettleMessageCommand, ShutdownBehavior,
    ShutdownProxyRequest, StartSubscriberCommand, StopPublisherRequest, StopSubscriberCommand,
    SubscriberCommand, SubscriberEvent, UnaryPullRequest, settle_message_command::SettleAction,
    settlement_result_event::AckStatus, start_subscriber_command::AutoSettleMode,
    subscriber_command, subscriber_event,
};
use pubsub_conformance_proxy::server::{
    ProxyService, ShutdownController, serve_with_grace_deadline,
};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::codegen::{Body, BoxFuture, Service, http};
use tonic::transport::{Endpoint, Server};
use tonic::{Code, Request, Response, Status, Streaming};

async fn spawn_in_process_proxy() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let shutdown_ctrl = ShutdownController::new();
    let svc = ProxyService::new(shutdown_ctrl);

    let handle = tokio::spawn(async move {
        serve_with_grace_deadline(listener, svc, std::future::pending::<()>())
            .await
            .expect("proxy server serves cleanly");
    });

    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn grpc_capabilities_unimplemented_and_validation_roundtrip() {
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let channel = Endpoint::from_shared(endpoint)
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to proxy");
    let mut client = PubSubProxyClient::new(channel);

    let caps = client
        .get_capabilities(GetCapabilitiesRequest {})
        .await
        .expect("GetCapabilities succeeds")
        .into_inner();
    assert_eq!(caps.client_language, "rust");
    assert_eq!(caps.client_library, "google-cloud-pubsub");
    assert!(!caps.supports_monitoring);
    assert!(!caps.supports_unary_pull);
    assert!(caps.supports_exactly_once_delivery);

    let pull_err = client
        .unary_pull(UnaryPullRequest::default())
        .await
        .unwrap_err();
    assert_eq!(pull_err.code(), Code::Unimplemented);

    let reset_err = client
        .reset_traces(ResetTracesRequest {})
        .await
        .unwrap_err();
    assert_eq!(reset_err.code(), Code::Unimplemented);

    let drain_err = client
        .drain_traces(DrainTracesRequest {})
        .await
        .unwrap_err();
    assert_eq!(drain_err.code(), Code::Unimplemented);

    let short_topic_err = client
        .create_publisher(CreatePublisherRequest {
            endpoint: "127.0.0.1:1".to_string(),
            topic: "short-only".to_string(),
            project_id: String::new(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(short_topic_err.code(), Code::InvalidArgument);

    let created = client
        .create_publisher(CreatePublisherRequest {
            endpoint: "127.0.0.1:1".to_string(),
            topic: "projects/test-proj/topics/t1".to_string(),
            ..Default::default()
        })
        .await
        .expect("CreatePublisher accepts full topic")
        .into_inner();

    let utf8_err = client
        .publish(PublishRequest {
            publisher_id: created.publisher_id.clone(),
            data: b"msg".to_vec(),
            raw_attributes: vec![RawAttribute {
                key: b"bad".to_vec(),
                value: vec![0xff, 0x00],
            }],
            ..Default::default()
        })
        .await
        .expect("PUB-02 returns in-band PublishResponse")
        .into_inner();
    assert_eq!(utf8_err.error_code, Code::InvalidArgument as i32);

    client
        .stop_publisher(StopPublisherRequest {
            publisher_id: created.publisher_id.clone(),
        })
        .await
        .expect("StopPublisher");

    let after_stop = client
        .publish(PublishRequest {
            publisher_id: created.publisher_id.clone(),
            data: b"msg".to_vec(),
            ..Default::default()
        })
        .await
        .expect("post-stop publish returns in-band error")
        .into_inner();
    assert_eq!(after_stop.error_code, Code::FailedPrecondition as i32);

    client
        .close_publisher(ClosePublisherRequest {
            publisher_id: created.publisher_id,
        })
        .await
        .expect("ClosePublisher");

    // Pre-start validation error on SubscriberSession (CFG-03 short subscription without project_id).
    let (cmd_tx, cmd_rx) = mpsc::channel(4);
    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Start(StartSubscriberCommand {
                endpoint: "127.0.0.1:1".to_string(),
                subscription: "short-sub".to_string(),
                project_id: String::new(),
                ..Default::default()
            })),
        })
        .await
        .expect("send start");
    drop(cmd_tx);
    let sub_err = client
        .subscriber_session(ReceiverStream::new(cmd_rx))
        .await
        .unwrap_err();
    assert_eq!(sub_err.code(), Code::InvalidArgument);

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy succeeds");

    tokio::time::timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("server stops within timeout")
        .expect("server task finishes cleanly");
}

#[tokio::test]
async fn binary_prints_ready_serves_grpc_and_shuts_down_cleanly() {
    let bin = env!("CARGO_BIN_EXE_pubsub-conformance-proxy");
    let mut child = Command::new(bin)
        .arg("--port=0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn proxy binary");

    let stdout = child.stdout.take().expect("piped stdout");
    let ready_line = tokio::task::spawn_blocking(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).expect("read READY line");
        line
    })
    .await
    .expect("read ready line task");

    let addr = ready_line
        .trim_end()
        .strip_prefix("READY ")
        .expect("READY line must start with 'READY '");
    assert!(
        addr.starts_with("127.0.0.1:"),
        "unexpected READY addr: {addr}"
    );

    let channel = Endpoint::from_shared(format!("http://{addr}"))
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to spawned binary");
    let mut client = PubSubProxyClient::new(channel);
    let caps = client
        .get_capabilities(GetCapabilitiesRequest {})
        .await
        .expect("GetCapabilities over spawned binary")
        .into_inner();
    assert_eq!(caps.client_language, "rust");

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy RPC");

    let status = tokio::task::spawn_blocking(move || child.wait())
        .await
        .expect("wait task")
        .expect("wait child");
    assert!(status.success(), "proxy exited with {status}");
}

#[tokio::test]
async fn publish_async_1005_throughput_smoke() {
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let channel = Endpoint::from_shared(endpoint)
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to proxy");
    let mut client = PubSubProxyClient::new(channel);

    let created = client
        .create_publisher(CreatePublisherRequest {
            endpoint: "127.0.0.1:1".to_string(),
            topic: "projects/test-proj/topics/t1".to_string(),
            count_threshold: Some(1000),
            byte_threshold: Some(32_000_000),
            delay_threshold_ms: Some(30_000),
            ..Default::default()
        })
        .await
        .expect("CreatePublisher")
        .into_inner();

    let start = std::time::Instant::now();
    for i in 0..1005 {
        let resp = client
            .publish(PublishRequest {
                publisher_id: created.publisher_id.clone(),
                data: format!("p03-{i:04}").into_bytes(),
                async_fire: true,
                ..Default::default()
            })
            .await
            .expect("publish async")
            .into_inner();
        assert_eq!(resp.error_code, 0);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "1005 async_fire calls took too long: {elapsed:?}"
    );

    client
        .close_publisher(ClosePublisherRequest {
            publisher_id: created.publisher_id,
        })
        .await
        .expect("ClosePublisher");

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

mod fake_pubsub {
    use std::convert::Infallible;

    use super::{
        Arc, AtomicBool, AtomicU64, AtomicUsize, Body, BoxFuture, Code, Context, HashMap, Mutex,
        Notify, Ordering, Poll, ReceiverStream, Request, Response, Server, Service, Status,
        Streaming, TcpListener, TcpListenerStream, VecDeque, http, mpsc,
    };

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PubsubMessage {
        #[prost(bytes = "vec", tag = "1")]
        pub data: Vec<u8>,
        #[prost(map = "string, string", tag = "2")]
        pub attributes: HashMap<String, String>,
        #[prost(string, tag = "3")]
        pub message_id: String,
        #[prost(message, optional, tag = "4")]
        pub publish_time: Option<prost_types::Timestamp>,
        #[prost(string, tag = "5")]
        pub ordering_key: String,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublishRequest {
        #[prost(string, tag = "1")]
        pub topic: String,
        #[prost(message, repeated, tag = "2")]
        pub messages: Vec<PubsubMessage>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublishResponse {
        #[prost(string, repeated, tag = "1")]
        pub message_ids: Vec<String>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ReceivedMessage {
        #[prost(string, tag = "1")]
        pub ack_id: String,
        #[prost(message, optional, tag = "2")]
        pub message: Option<PubsubMessage>,
        #[prost(int32, tag = "3")]
        pub delivery_attempt: i32,
    }

    #[derive(Clone, Copy, PartialEq, Eq, ::prost::Message)]
    pub struct SubscriptionProperties {
        #[prost(bool, tag = "1")]
        pub exactly_once_delivery_enabled: bool,
        #[prost(bool, tag = "2")]
        pub message_ordering_enabled: bool,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StreamingPullRequest {
        #[prost(string, tag = "1")]
        pub subscription: String,
        #[prost(string, repeated, tag = "2")]
        pub ack_ids: Vec<String>,
        #[prost(int32, repeated, tag = "3")]
        pub modify_deadline_seconds: Vec<i32>,
        #[prost(string, repeated, tag = "4")]
        pub modify_deadline_ack_ids: Vec<String>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StreamingPullResponse {
        #[prost(message, repeated, tag = "1")]
        pub received_messages: Vec<ReceivedMessage>,
        #[prost(message, optional, tag = "4")]
        pub subscription_properties: Option<SubscriptionProperties>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AcknowledgeRequest {
        #[prost(string, tag = "1")]
        pub subscription: String,
        #[prost(string, repeated, tag = "2")]
        pub ack_ids: Vec<String>,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ModifyAckDeadlineRequest {
        #[prost(string, tag = "1")]
        pub subscription: String,
        #[prost(int32, tag = "3")]
        pub ack_deadline_seconds: i32,
        #[prost(string, repeated, tag = "4")]
        pub ack_ids: Vec<String>,
    }

    #[derive(Default)]
    pub struct FakePubsubConfig {
        pub exactly_once: bool,
        pub defer_first_pull_headers: bool,
        pub gate_first_publish: bool,
    }

    pub struct FakePubsubState {
        pub exactly_once: bool,
        pub defer_first_pull_headers: AtomicBool,
        pub first_pull_header_entered: Notify,
        pub first_pull_header_release: Notify,
        pub gate_first_publish: AtomicBool,
        pub first_publish_entered: Notify,
        pub first_publish_release: Notify,
        pub publish_count: AtomicUsize,
        pub publish_seq: AtomicU64,
        pub published_messages: Mutex<Vec<(String, PubsubMessage)>>,
        pub publish_error: Mutex<Option<(Code, String)>>,
        pub streaming_pull_open_count: AtomicUsize,
        pub active_stream_tx: Mutex<Option<mpsc::Sender<Result<StreamingPullResponse, Status>>>>,
        pub queued_responses: Mutex<VecDeque<StreamingPullResponse>>,
        pub ack_error: Mutex<Option<(Code, String)>>,
        pub nack_error: Mutex<Option<(Code, String)>>,
        pub acked_ids: Mutex<Vec<String>>,
        pub nacked_ids: Mutex<Vec<String>>,
        pub activity: Notify,
    }

    impl FakePubsubState {
        fn new(cfg: FakePubsubConfig) -> Self {
            Self {
                exactly_once: cfg.exactly_once,
                defer_first_pull_headers: AtomicBool::new(cfg.defer_first_pull_headers),
                first_pull_header_entered: Notify::new(),
                first_pull_header_release: Notify::new(),
                gate_first_publish: AtomicBool::new(cfg.gate_first_publish),
                first_publish_entered: Notify::new(),
                first_publish_release: Notify::new(),
                publish_count: AtomicUsize::new(0),
                publish_seq: AtomicU64::new(0),
                published_messages: Mutex::new(Vec::new()),
                publish_error: Mutex::new(None),
                streaming_pull_open_count: AtomicUsize::new(0),
                active_stream_tx: Mutex::new(None),
                queued_responses: Mutex::new(VecDeque::new()),
                ack_error: Mutex::new(None),
                nack_error: Mutex::new(None),
                acked_ids: Mutex::new(Vec::new()),
                nacked_ids: Mutex::new(Vec::new()),
                activity: Notify::new(),
            }
        }

        pub async fn push_message(&self, ack_id: &str, message_id: &str, data: &[u8]) {
            let resp = StreamingPullResponse {
                received_messages: vec![ReceivedMessage {
                    ack_id: ack_id.to_string(),
                    message: Some(PubsubMessage {
                        data: data.to_vec(),
                        attributes: HashMap::new(),
                        message_id: message_id.to_string(),
                        publish_time: None,
                        ordering_key: String::new(),
                    }),
                    delivery_attempt: 1,
                }],
                subscription_properties: Some(SubscriptionProperties {
                    exactly_once_delivery_enabled: self.exactly_once,
                    message_ordering_enabled: false,
                }),
            };
            let tx = self.active_stream_tx.lock().await.clone();
            if let Some(tx) = tx
                && tx.send(Ok(resp.clone())).await.is_ok()
            {
                return;
            }
            self.queued_responses.lock().await.push_back(resp);
        }

        pub async fn wait_for_pull_open(&self, min_count: usize) {
            loop {
                let notified = self.activity.notified();
                if self.streaming_pull_open_count.load(Ordering::SeqCst) >= min_count {
                    return;
                }
                notified.await;
            }
        }

        pub async fn wait_for_acks(&self, min_count: usize) -> Vec<String> {
            loop {
                let notified = self.activity.notified();
                {
                    let acked = self.acked_ids.lock().await;
                    if acked.len() >= min_count {
                        return acked.clone();
                    }
                }
                notified.await;
            }
        }

        pub async fn wait_for_nacks(&self, min_count: usize) -> Vec<String> {
            loop {
                let notified = self.activity.notified();
                {
                    let nacked = self.nacked_ids.lock().await;
                    if nacked.len() >= min_count {
                        return nacked.clone();
                    }
                }
                notified.await;
            }
        }
    }

    pub struct FakePubsubServer {
        pub addr: String,
        pub state: Arc<FakePubsubState>,
        shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
        handle: Option<tokio::task::JoinHandle<()>>,
    }

    impl Drop for FakePubsubServer {
        fn drop(&mut self) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
            if let Some(handle) = self.handle.take() {
                handle.abort();
            }
        }
    }

    #[derive(Clone)]
    struct FakePublisherSvc {
        state: Arc<FakePubsubState>,
    }

    impl tonic::server::NamedService for FakePublisherSvc {
        const NAME: &'static str = "google.pubsub.v1.Publisher";
    }

    impl<B> Service<http::Request<B>> for FakePublisherSvc
    where
        B: Body + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let state = Arc::clone(&self.state);
            match req.uri().path() {
                "/google.pubsub.v1.Publisher/Publish" => {
                    struct PublishUnary(Arc<FakePubsubState>);
                    impl tonic::server::UnaryService<PublishRequest> for PublishUnary {
                        type Response = PublishResponse;
                        type Future = BoxFuture<Response<Self::Response>, Status>;

                        fn call(&mut self, request: Request<PublishRequest>) -> Self::Future {
                            let state = Arc::clone(&self.0);
                            Box::pin(async move {
                                let call_idx = state.publish_count.fetch_add(1, Ordering::SeqCst);
                                state.activity.notify_waiters();
                                if call_idx == 0 && state.gate_first_publish.load(Ordering::SeqCst)
                                {
                                    state.first_publish_entered.notify_waiters();
                                    state.first_publish_release.notified().await;
                                }
                                if let Some((code, msg)) = state.publish_error.lock().await.clone()
                                {
                                    return Err(Status::new(code, msg));
                                }
                                let inner = request.into_inner();
                                let mut ids = Vec::with_capacity(inner.messages.len());
                                let mut recorded = state.published_messages.lock().await;
                                for msg in inner.messages {
                                    let seq = state.publish_seq.fetch_add(1, Ordering::SeqCst) + 1;
                                    ids.push(format!("fake-msg-{seq}"));
                                    recorded.push((inner.topic.clone(), msg));
                                }
                                Ok(Response::new(PublishResponse { message_ids: ids }))
                            })
                        }
                    }

                    Box::pin(async move {
                        let codec: tonic_prost::ProstCodec<PublishResponse, PublishRequest> =
                            tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        Ok(grpc.unary(PublishUnary(state), req).await)
                    })
                }
                _ => unimplemented_grpc(),
            }
        }
    }

    #[derive(Clone)]
    struct FakeSubscriberSvc {
        state: Arc<FakePubsubState>,
    }

    impl tonic::server::NamedService for FakeSubscriberSvc {
        const NAME: &'static str = "google.pubsub.v1.Subscriber";
    }

    impl<B> Service<http::Request<B>> for FakeSubscriberSvc
    where
        B: Body + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let state = Arc::clone(&self.state);
            match req.uri().path() {
                "/google.pubsub.v1.Subscriber/StreamingPull" => {
                    struct StreamingPullImpl(Arc<FakePubsubState>);
                    impl tonic::server::StreamingService<StreamingPullRequest> for StreamingPullImpl {
                        type Response = StreamingPullResponse;
                        type ResponseStream = ReceiverStream<Result<StreamingPullResponse, Status>>;
                        type Future = BoxFuture<Response<Self::ResponseStream>, Status>;

                        fn call(
                            &mut self,
                            request: Request<Streaming<StreamingPullRequest>>,
                        ) -> Self::Future {
                            let state = Arc::clone(&self.0);
                            Box::pin(async move {
                                let open_idx = state
                                    .streaming_pull_open_count
                                    .fetch_add(1, Ordering::SeqCst);
                                state.activity.notify_waiters();
                                if open_idx == 0
                                    && state.defer_first_pull_headers.load(Ordering::SeqCst)
                                {
                                    state.first_pull_header_entered.notify_waiters();
                                    state.first_pull_header_release.notified().await;
                                }

                                let (tx, rx) = mpsc::channel(64);
                                {
                                    let mut queued = state.queued_responses.lock().await;
                                    while let Some(resp) = queued.pop_front() {
                                        let _ = tx.try_send(Ok(resp));
                                    }
                                    *state.active_stream_tx.lock().await = Some(tx);
                                }

                                let mut in_stream = request.into_inner();
                                let inbound_state = Arc::clone(&state);
                                tokio::spawn(async move {
                                    while let Ok(Some(msg)) = in_stream.message().await {
                                        let mut changed = false;
                                        if !msg.ack_ids.is_empty() {
                                            inbound_state
                                                .acked_ids
                                                .lock()
                                                .await
                                                .extend(msg.ack_ids);
                                            changed = true;
                                        }
                                        for (ack_id, secs) in msg
                                            .modify_deadline_ack_ids
                                            .into_iter()
                                            .zip(msg.modify_deadline_seconds)
                                        {
                                            if secs == 0 {
                                                inbound_state.nacked_ids.lock().await.push(ack_id);
                                                changed = true;
                                            }
                                        }
                                        if changed {
                                            inbound_state.activity.notify_waiters();
                                        }
                                    }
                                });

                                Ok(Response::new(ReceiverStream::new(rx)))
                            })
                        }
                    }

                    Box::pin(async move {
                        let codec: tonic_prost::ProstCodec<
                            StreamingPullResponse,
                            StreamingPullRequest,
                        > = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        Ok(grpc.streaming(StreamingPullImpl(state), req).await)
                    })
                }
                "/google.pubsub.v1.Subscriber/Acknowledge" => {
                    struct AcknowledgeUnary(Arc<FakePubsubState>);
                    impl tonic::server::UnaryService<AcknowledgeRequest> for AcknowledgeUnary {
                        type Response = ();
                        type Future = BoxFuture<Response<Self::Response>, Status>;

                        fn call(&mut self, request: Request<AcknowledgeRequest>) -> Self::Future {
                            let state = Arc::clone(&self.0);
                            Box::pin(async move {
                                if let Some((code, msg)) = state.ack_error.lock().await.clone() {
                                    return Err(Status::new(code, msg));
                                }
                                let req = request.into_inner();
                                state.acked_ids.lock().await.extend(req.ack_ids);
                                state.activity.notify_waiters();
                                Ok(Response::new(()))
                            })
                        }
                    }

                    Box::pin(async move {
                        let codec: tonic_prost::ProstCodec<(), AcknowledgeRequest> =
                            tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        Ok(grpc.unary(AcknowledgeUnary(state), req).await)
                    })
                }
                "/google.pubsub.v1.Subscriber/ModifyAckDeadline" => {
                    struct ModAckUnary(Arc<FakePubsubState>);
                    impl tonic::server::UnaryService<ModifyAckDeadlineRequest> for ModAckUnary {
                        type Response = ();
                        type Future = BoxFuture<Response<Self::Response>, Status>;

                        fn call(
                            &mut self,
                            request: Request<ModifyAckDeadlineRequest>,
                        ) -> Self::Future {
                            let state = Arc::clone(&self.0);
                            Box::pin(async move {
                                let req = request.into_inner();
                                if req.ack_deadline_seconds == 0 {
                                    if let Some((code, msg)) = state.nack_error.lock().await.clone()
                                    {
                                        return Err(Status::new(code, msg));
                                    }
                                    state.nacked_ids.lock().await.extend(req.ack_ids);
                                    state.activity.notify_waiters();
                                }
                                Ok(Response::new(()))
                            })
                        }
                    }

                    Box::pin(async move {
                        let codec: tonic_prost::ProstCodec<(), ModifyAckDeadlineRequest> =
                            tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec);
                        Ok(grpc.unary(ModAckUnary(state), req).await)
                    })
                }
                _ => unimplemented_grpc(),
            }
        }
    }

    fn unimplemented_grpc() -> BoxFuture<http::Response<tonic::body::Body>, Infallible> {
        Box::pin(async move {
            let mut response = http::Response::new(tonic::body::Body::default());
            let headers = response.headers_mut();
            headers.insert(
                tonic::Status::GRPC_STATUS,
                (tonic::Code::Unimplemented as i32).into(),
            );
            headers.insert(
                http::header::CONTENT_TYPE,
                tonic::metadata::GRPC_CONTENT_TYPE,
            );
            Ok(response)
        })
    }

    pub async fn start(cfg: FakePubsubConfig) -> FakePubsubServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fake pubsub");
        let addr = listener.local_addr().expect("fake pubsub addr").to_string();
        let state = Arc::new(FakePubsubState::new(cfg));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let incoming = TcpListenerStream::new(listener);
        let pub_svc = FakePublisherSvc {
            state: Arc::clone(&state),
        };
        let sub_svc = FakeSubscriberSvc {
            state: Arc::clone(&state),
        };
        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(pub_svc)
                .add_service(sub_svc)
                .serve_with_incoming_shutdown(incoming, async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("fake pubsub serves cleanly");
        });
        FakePubsubServer {
            addr,
            state,
            shutdown_tx: Some(shutdown_tx),
            handle: Some(handle),
        }
    }
}

async fn connect_proxy_client(endpoint: &str) -> PubSubProxyClient<tonic::transport::Channel> {
    let channel = Endpoint::from_shared(endpoint.to_string())
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to proxy");
    PubSubProxyClient::new(channel)
}

async fn recv_event_timeout(
    stream: &mut Streaming<SubscriberEvent>,
    timeout: Duration,
) -> Option<SubscriberEvent> {
    tokio::time::timeout(timeout, stream.message())
        .await
        .expect("timed out waiting for SubscriberEvent")
        .expect("SubscriberEvent stream returned RPC error")
}

async fn open_subscriber_session(
    client: &mut PubSubProxyClient<tonic::transport::Channel>,
    start_cmd: StartSubscriberCommand,
) -> (
    mpsc::Sender<SubscriberCommand>,
    Streaming<SubscriberEvent>,
    String,
) {
    let (cmd_tx, cmd_rx) = mpsc::channel(16);
    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Start(start_cmd)),
        })
        .await
        .expect("send StartSubscriberCommand");
    let mut event_stream = client
        .subscriber_session(ReceiverStream::new(cmd_rx))
        .await
        .expect("open SubscriberSession")
        .into_inner();
    let first = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected StartedEvent");
    let client_id = match first.event {
        Some(subscriber_event::Event::Started(started)) => started.client_id,
        other => panic!("expected StartedEvent, got {other:?}"),
    };
    (cmd_tx, event_stream, client_id)
}

fn spawn_proxy_binary(envs: &[(&str, &str)]) -> (Child, String) {
    let bin = env!("CARGO_BIN_EXE_pubsub-conformance-proxy");
    let mut cmd = Command::new(bin);
    cmd.arg("--port=0")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn proxy binary");
    let stdout = child.stdout.take().expect("piped stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read READY line");
    let addr = line
        .trim_end()
        .strip_prefix("READY ")
        .expect("READY line must start with 'READY '")
        .to_string();
    (child, addr)
}

#[tokio::test]
async fn shutdown_proxy_with_open_subscriber_session_exits_within_grace_period() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-1".to_string(),
            auto_settle_mode: AutoSettleMode::Interactive as i32,
            ..Default::default()
        },
    )
    .await;
    fake.state.wait_for_pull_open(1).await;

    // Keep client, cmd_tx, and event_stream open across ShutdownProxy.
    let started_shutdown = Instant::now();
    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 150,
        })
        .await
        .expect("ShutdownProxy RPC succeeds");

    let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(1))
        .await
        .expect("expected StoppedEvent on global shutdown");
    match ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }
    assert!(
        recv_event_timeout(&mut event_stream, Duration::from_secs(1))
            .await
            .is_none(),
        "SubscriberSession stream must close after StoppedEvent"
    );

    tokio::time::timeout(Duration::from_secs(1), server_handle)
        .await
        .expect("server exits within bounded grace period even with open client")
        .expect("server task finishes cleanly");
    assert!(
        started_shutdown.elapsed() < Duration::from_secs(1),
        "shutdown took too long: {:?}",
        started_shutdown.elapsed()
    );
    drop(cmd_tx);
    drop(client);
}

#[cfg(unix)]
#[tokio::test]
async fn sigterm_with_open_subscriber_session_exits_cleanly() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (mut child, addr) = spawn_proxy_binary(&[]);
    let mut client = connect_proxy_client(&format!("http://{addr}")).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-sigterm".to_string(),
            auto_settle_mode: AutoSettleMode::Interactive as i32,
            ..Default::default()
        },
    )
    .await;
    fake.state.wait_for_pull_open(1).await;

    let pid = child.id().to_string();
    let started_sig = Instant::now();
    let kill_status = Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .expect("invoke kill -TERM");
    assert!(kill_status.success(), "kill -TERM failed: {kill_status}");

    let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(1))
        .await
        .expect("expected StoppedEvent on SIGTERM");
    match ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }
    assert!(
        recv_event_timeout(&mut event_stream, Duration::from_secs(1))
            .await
            .is_none()
    );

    let wait_status = tokio::time::timeout(
        Duration::from_millis(1500),
        tokio::task::spawn_blocking(move || child.wait()),
    )
    .await
    .expect("proxy binary exits promptly on SIGTERM")
    .expect("wait task")
    .expect("child wait");
    assert!(
        wait_status.success(),
        "expected exit code 0 on SIGTERM, got {wait_status}"
    );
    assert!(
        started_sig.elapsed() < Duration::from_millis(1500),
        "SIGTERM drain exceeded bound: {:?}",
        started_sig.elapsed()
    );
    drop(cmd_tx);
    drop(client);
}

#[tokio::test]
async fn settle_command_during_deferred_streaming_pull_headers_does_not_reopen_stream() {
    let fake = fake_pubsub::start(fake_pubsub::FakePubsubConfig {
        defer_first_pull_headers: true,
        ..Default::default()
    })
    .await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-extra1".to_string(),
            auto_settle_mode: AutoSettleMode::Interactive as i32,
            ..Default::default()
        },
    )
    .await;

    fake.state.first_pull_header_entered.notified().await;

    // Wake select! via SettleMessageCommand multiple times while open_stream() awaits headers.
    for token in ["unknown-1", "unknown-2"] {
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Settle(SettleMessageCommand {
                    delivery_token: token.to_string(),
                    action: SettleAction::Ack as i32,
                })),
            })
            .await
            .expect("send SettleMessageCommand during header stall");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    fake.state.first_pull_header_release.notify_waiters();
    fake.state
        .push_message("ack-extra1", "msg-extra1", b"hello")
        .await;

    let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected MessageDeliveredEvent");
    let token = match ev.event {
        Some(subscriber_event::Event::MessageDelivered(msg)) => {
            assert_eq!(msg.message_id, "msg-extra1");
            msg.delivery_token
        }
        other => panic!("expected MessageDeliveredEvent, got {other:?}"),
    };

    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Settle(SettleMessageCommand {
                delivery_token: token,
                action: SettleAction::Ack as i32,
            })),
        })
        .await
        .expect("ack delivered message");
    assert_eq!(fake.state.wait_for_acks(1).await, vec!["ack-extra1"]);

    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Stop(StopSubscriberCommand {
                shutdown_behavior: ShutdownBehavior::WaitForProcessing as i32,
                shutdown_timeout_ms: None,
            })),
        })
        .await
        .expect("send StopSubscriberCommand");
    let stop_ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected StoppedEvent");
    match stop_ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
            assert_eq!(stopped.total_delivered, 1);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }

    assert_eq!(
        fake.state.streaming_pull_open_count.load(Ordering::SeqCst),
        1,
        "StreamingPull must open only once across select! wakes during header stall"
    );

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn interactive_close_send_with_in_flight_auto_nacks_and_emits_stopped() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-2a".to_string(),
            auto_settle_mode: AutoSettleMode::Interactive as i32,
            ..Default::default()
        },
    )
    .await;

    fake.state
        .push_message("ack-2a", "msg-2a", b"payload")
        .await;
    let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected MessageDeliveredEvent");
    assert!(matches!(
        ev.event,
        Some(subscriber_event::Event::MessageDelivered(_))
    ));

    // CloseSend() with unsettled in-flight message must abort deadlock, auto-nack, and emit StoppedEvent.
    drop(cmd_tx);

    let stop_ev = recv_event_timeout(&mut event_stream, Duration::from_secs(2))
        .await
        .expect("expected StoppedEvent after CloseSend");
    match stop_ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
            assert_eq!(stopped.total_delivered, 1);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }
    assert!(
        recv_event_timeout(&mut event_stream, Duration::from_secs(1))
            .await
            .is_none()
    );
    assert_eq!(fake.state.wait_for_nacks(1).await, vec!["ack-2a"]);

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn auto_ack_close_send_continues_delivering_messages() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-2b".to_string(),
            auto_settle_mode: AutoSettleMode::AutoAck as i32,
            ..Default::default()
        },
    )
    .await;
    fake.state.wait_for_pull_open(1).await;

    // CloseSend() in AUTO_* mode must keep delivering messages.
    drop(cmd_tx);
    tokio::time::sleep(Duration::from_millis(30)).await;

    fake.state
        .push_message("ack-2b-1", "msg-2b-1", b"one")
        .await;
    fake.state
        .push_message("ack-2b-2", "msg-2b-2", b"two")
        .await;

    let mut delivered = Vec::new();
    for _ in 0..2 {
        let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .expect("expected MessageDeliveredEvent after CloseSend");
        match ev.event {
            Some(subscriber_event::Event::MessageDelivered(msg)) => {
                delivered.push(msg.message_id);
            }
            other => panic!("expected MessageDeliveredEvent, got {other:?}"),
        }
    }
    assert_eq!(delivered, vec!["msg-2b-1", "msg-2b-2"]);

    let acked = fake.state.wait_for_acks(2).await;
    assert!(acked.contains(&"ack-2b-1".to_string()));
    assert!(acked.contains(&"ack-2b-2".to_string()));

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 200,
        })
        .await
        .expect("ShutdownProxy");
    let stop_ev = recv_event_timeout(&mut event_stream, Duration::from_secs(2))
        .await
        .expect("expected StoppedEvent on proxy shutdown");
    match stop_ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
            assert_eq!(stopped.total_delivered, 2);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn all_auto_settle_modes_deliver_and_settle() {
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    // 1. AutoAck
    {
        let fake = fake_pubsub::start(Default::default()).await;
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/auto-ack".to_string(),
                auto_settle_mode: AutoSettleMode::AutoAck as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state.push_message("ack-aa", "msg-aa", b"aa").await;
        let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(matches!(
            ev.event,
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        assert_eq!(fake.state.wait_for_acks(1).await, vec!["ack-aa"]);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::Stopped(_))
        ));
    }

    // 2. AutoNack
    {
        let fake = fake_pubsub::start(Default::default()).await;
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/auto-nack".to_string(),
                auto_settle_mode: AutoSettleMode::AutoNack as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state.push_message("ack-an", "msg-an", b"an").await;
        let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(matches!(
            ev.event,
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        assert_eq!(fake.state.wait_for_nacks(1).await, vec!["ack-an"]);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::Stopped(_))
        ));
    }

    // 3. AutoDropWithoutSettle
    {
        let fake = fake_pubsub::start(Default::default()).await;
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/auto-drop".to_string(),
                auto_settle_mode: AutoSettleMode::AutoDropWithoutSettle as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state.push_message("ack-ad", "msg-ad", b"ad").await;
        let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .unwrap();
        assert!(matches!(
            ev.event,
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        assert_eq!(fake.state.wait_for_nacks(1).await, vec!["ack-ad"]);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::Stopped(_))
        ));
    }

    // 4. AutoAckWithResult (success + PermissionDenied)
    {
        let fake = fake_pubsub::start(fake_pubsub::FakePubsubConfig {
            exactly_once: true,
            ..Default::default()
        })
        .await;
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/auto-ack-res".to_string(),
                use_exactly_once: true,
                auto_settle_mode: AutoSettleMode::AutoAckWithResult as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state.push_message("ack-ar1", "msg-ar1", b"ar1").await;
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        match recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .and_then(|e| e.event)
        {
            Some(subscriber_event::Event::SettlementResult(res)) => {
                assert_eq!(res.status, AckStatus::Success as i32);
                assert_eq!(res.grpc_error_code, 0);
                assert!(!res.is_nack);
            }
            other => panic!("expected SettlementResult(Success), got {other:?}"),
        }

        *fake.state.ack_error.lock().await = Some((Code::PermissionDenied, "denied".to_string()));
        fake.state.push_message("ack-ar2", "msg-ar2", b"ar2").await;
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        match recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .and_then(|e| e.event)
        {
            Some(subscriber_event::Event::SettlementResult(res)) => {
                assert_eq!(res.status, AckStatus::PermissionDenied as i32);
                assert_eq!(res.grpc_error_code, Code::PermissionDenied as i32);
                assert!(!res.is_nack);
            }
            other => panic!("expected SettlementResult(PermissionDenied), got {other:?}"),
        }
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::Stopped(_))
        ));
    }

    // 5. AutoNackWithResult (success + PermissionDenied)
    {
        let fake = fake_pubsub::start(fake_pubsub::FakePubsubConfig {
            exactly_once: true,
            ..Default::default()
        })
        .await;
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/auto-nack-res".to_string(),
                use_exactly_once: true,
                auto_settle_mode: AutoSettleMode::AutoNackWithResult as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state.push_message("ack-nr1", "msg-nr1", b"nr1").await;
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        match recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .and_then(|e| e.event)
        {
            Some(subscriber_event::Event::SettlementResult(res)) => {
                assert_eq!(res.status, AckStatus::Success as i32);
                assert_eq!(res.grpc_error_code, 0);
                assert!(res.is_nack);
            }
            other => panic!("expected SettlementResult(Success), got {other:?}"),
        }

        *fake.state.nack_error.lock().await =
            Some((Code::PermissionDenied, "nack denied".to_string()));
        fake.state.push_message("ack-nr2", "msg-nr2", b"nr2").await;
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        match recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .and_then(|e| e.event)
        {
            Some(subscriber_event::Event::SettlementResult(res)) => {
                assert_eq!(res.status, AckStatus::PermissionDenied as i32);
                assert_eq!(res.grpc_error_code, Code::PermissionDenied as i32);
                assert!(res.is_nack);
            }
            other => panic!("expected SettlementResult(PermissionDenied), got {other:?}"),
        }
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        assert!(matches!(
            recv_event_timeout(&mut event_stream, Duration::from_secs(5))
                .await
                .and_then(|e| e.event),
            Some(subscriber_event::Event::Stopped(_))
        ));
    }

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn concurrent_stop_publisher_and_flush_wait_for_gated_drain() {
    let fake = fake_pubsub::start(fake_pubsub::FakePubsubConfig {
        gate_first_publish: true,
        ..Default::default()
    })
    .await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let created = client
        .create_publisher(CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/test-proj/topics/gated-pub".to_string(),
            count_threshold: Some(1),
            ..Default::default()
        })
        .await
        .expect("CreatePublisher")
        .into_inner();

    let pub_resp = client
        .publish(PublishRequest {
            publisher_id: created.publisher_id.clone(),
            data: b"gated".to_vec(),
            async_fire: true,
            ..Default::default()
        })
        .await
        .expect("async_fire Publish")
        .into_inner();
    assert!(!pub_resp.publish_token.is_empty());

    fake.state.first_publish_entered.notified().await;

    let mut c1 = client.clone();
    let id1 = created.publisher_id.clone();
    let t1 = tokio::spawn(async move {
        c1.stop_publisher(StopPublisherRequest { publisher_id: id1 })
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;

    let mut c2 = client.clone();
    let id2 = created.publisher_id.clone();
    let t2 = tokio::spawn(async move {
        c2.stop_publisher(StopPublisherRequest { publisher_id: id2 })
            .await
    });

    let mut c3 = client.clone();
    let id3 = created.publisher_id.clone();
    let t3 = tokio::spawn(async move {
        c3.flush_publisher(FlushPublisherRequest { publisher_id: id3 })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!t1.is_finished(), "StopPublisher #1 returned before drain");
    assert!(
        !t2.is_finished(),
        "concurrent StopPublisher #2 returned before active drain"
    );
    assert!(
        !t3.is_finished(),
        "concurrent FlushPublisher returned before active drain"
    );

    fake.state.first_publish_release.notify_waiters();

    tokio::time::timeout(Duration::from_secs(2), t1)
        .await
        .expect("t1 finished")
        .expect("join t1")
        .expect("StopPublisher #1 Ok");
    tokio::time::timeout(Duration::from_secs(2), t2)
        .await
        .expect("t2 finished")
        .expect("join t2")
        .expect("StopPublisher #2 Ok");
    tokio::time::timeout(Duration::from_secs(2), t3)
        .await
        .expect("t3 finished")
        .expect("join t3")
        .expect("FlushPublisher Ok");

    let awaited = client
        .await_publish(AwaitPublishRequest {
            publisher_id: created.publisher_id,
            publish_token: pub_resp.publish_token,
        })
        .await
        .expect("AwaitPublish")
        .into_inner();
    assert_eq!(awaited.error_code, 0);
    assert_eq!(awaited.message_id, "fake-msg-1");

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn start_nack_immediately_override_stop_wait_for_processing_waits_for_ack() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let (cmd_tx, mut event_stream, _) = open_subscriber_session(
        &mut client,
        StartSubscriberCommand {
            endpoint: fake.addr.clone(),
            subscription: "projects/test-proj/subscriptions/sub-3".to_string(),
            shutdown_behavior: ShutdownBehavior::NackImmediately as i32,
            auto_settle_mode: AutoSettleMode::Interactive as i32,
            ..Default::default()
        },
    )
    .await;

    fake.state
        .push_message("ack-sub3", "msg-sub3", b"sub3")
        .await;
    let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected MessageDeliveredEvent");
    let token = match ev.event {
        Some(subscriber_event::Event::MessageDelivered(m)) => m.delivery_token,
        other => panic!("expected MessageDeliveredEvent, got {other:?}"),
    };

    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Stop(StopSubscriberCommand {
                shutdown_behavior: ShutdownBehavior::WaitForProcessing as i32,
                shutdown_timeout_ms: None,
            })),
        })
        .await
        .expect("send StopSubscriberCommand override");

    assert!(
        tokio::time::timeout(Duration::from_millis(60), event_stream.message())
            .await
            .is_err(),
        "Stop(WAIT_FOR_PROCESSING) must wait while message is in flight"
    );
    assert!(
        fake.state.nacked_ids.lock().await.is_empty(),
        "message must not be nacked while Stop(WAIT_FOR_PROCESSING) awaits ACK"
    );

    cmd_tx
        .send(SubscriberCommand {
            command: Some(subscriber_command::Command::Settle(SettleMessageCommand {
                delivery_token: token,
                action: SettleAction::Ack as i32,
            })),
        })
        .await
        .expect("send ACK after Stop");

    assert_eq!(fake.state.wait_for_acks(1).await, vec!["ack-sub3"]);

    let stop_ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
        .await
        .expect("expected StoppedEvent after ACK");
    match stop_ev.event {
        Some(subscriber_event::Event::Stopped(stopped)) => {
            assert_eq!(stopped.error_code, 0);
            assert_eq!(stopped.total_delivered, 1);
        }
        other => panic!("expected StoppedEvent, got {other:?}"),
    }
    assert!(fake.state.nacked_ids.lock().await.is_empty());

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn emulator_env_live_and_whitespace_rejected() {
    // Whitespace-only PUBSUB_EMULATOR_HOST must be rejected with FailedPrecondition.
    {
        let (mut child, addr) = spawn_proxy_binary(&[("PUBSUB_EMULATOR_HOST", "  \t ")]);
        let mut client = connect_proxy_client(&format!("http://{addr}")).await;

        let pub_err = client
            .create_publisher(CreatePublisherRequest {
                endpoint: String::new(),
                topic: "projects/test-proj/topics/t-env".to_string(),
                use_emulator_env: true,
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(pub_err.code(), Code::FailedPrecondition);

        let (cmd_tx, cmd_rx) = mpsc::channel(4);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Start(StartSubscriberCommand {
                    endpoint: String::new(),
                    subscription: "projects/test-proj/subscriptions/s-env".to_string(),
                    use_emulator_env: true,
                    ..Default::default()
                })),
            })
            .await
            .unwrap();
        drop(cmd_tx);
        let sub_err = client
            .subscriber_session(ReceiverStream::new(cmd_rx))
            .await
            .unwrap_err();
        assert_eq!(sub_err.code(), Code::FailedPrecondition);

        client
            .shutdown_proxy(ShutdownProxyRequest {
                grace_period_ms: 1000,
            })
            .await
            .expect("ShutdownProxy");
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }

    // Valid PUBSUB_EMULATOR_HOST enables live Publisher & Subscriber roundtrips.
    {
        let fake = fake_pubsub::start(Default::default()).await;
        let (mut child, addr) = spawn_proxy_binary(&[("PUBSUB_EMULATOR_HOST", &fake.addr)]);
        let mut client = connect_proxy_client(&format!("http://{addr}")).await;

        let created = client
            .create_publisher(CreatePublisherRequest {
                endpoint: String::new(),
                topic: "t-live".to_string(),
                project_id: "p-live".to_string(),
                use_emulator_env: true,
                count_threshold: Some(1),
                ..Default::default()
            })
            .await
            .expect("CreatePublisher using PUBSUB_EMULATOR_HOST")
            .into_inner();
        let pub_resp = client
            .publish(PublishRequest {
                publisher_id: created.publisher_id.clone(),
                data: b"via-env".to_vec(),
                ..Default::default()
            })
            .await
            .expect("Publish via PUBSUB_EMULATOR_HOST")
            .into_inner();
        assert_eq!(pub_resp.error_code, 0);
        assert_eq!(pub_resp.message_id, "fake-msg-1");

        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: String::new(),
                subscription: "s-live".to_string(),
                project_id: "p-live".to_string(),
                use_emulator_env: true,
                auto_settle_mode: AutoSettleMode::AutoAck as i32,
                ..Default::default()
            },
        )
        .await;
        fake.state
            .push_message("ack-env", "msg-env", b"from-env")
            .await;
        let ev = recv_event_timeout(&mut event_stream, Duration::from_secs(5))
            .await
            .expect("expected message from emulator env");
        assert!(matches!(
            ev.event,
            Some(subscriber_event::Event::MessageDelivered(_))
        ));
        assert_eq!(fake.state.wait_for_acks(1).await, vec!["ack-env"]);

        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(
                    StopSubscriberCommand::default(),
                )),
            })
            .await
            .unwrap();
        let _ = recv_event_timeout(&mut event_stream, Duration::from_secs(5)).await;

        client
            .shutdown_proxy(ShutdownProxyRequest {
                grace_period_ms: 1000,
            })
            .await
            .expect("ShutdownProxy");
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }
}

#[tokio::test]
async fn publisher_validation_blocking_not_found_and_zero_timeout() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    let invalid_cases = vec![
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "bad/short".to_string(),
            project_id: "p".to_string(),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            total_timeout_ms: Some(-1),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            max_attempts: Some(-1),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            retry_delay_multiplier: Some(f64::NAN),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            retry_delay_multiplier: Some(f64::INFINITY),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            retry_delay_multiplier: Some(0.5),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            initial_retry_delay_ms: Some(0),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            max_retry_delay_ms: Some(0),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            initial_retry_delay_ms: Some(500),
            max_retry_delay_ms: Some(100),
            ..Default::default()
        },
        CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/p/topics/t".to_string(),
            hedging: Some(HedgingSettings {
                enabled: true,
                delay_ms: Some(-1),
                ..Default::default()
            }),
            ..Default::default()
        },
    ];
    for req in invalid_cases {
        let err = client.create_publisher(req).await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    // NotFound RPCs for missing publisher_id.
    assert_eq!(
        client
            .publish(PublishRequest {
                publisher_id: "missing".to_string(),
                data: b"x".to_vec(),
                ..Default::default()
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .await_publish(AwaitPublishRequest {
                publisher_id: "missing".to_string(),
                publish_token: "tok".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .flush_publisher(FlushPublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .stop_publisher(StopPublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .resume_publish(ResumePublishRequest {
                publisher_id: "missing".to_string(),
                ordering_key: "k".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );
    assert_eq!(
        client
            .close_publisher(ClosePublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
        Code::NotFound
    );

    // PUB_PROBE_6B: total_timeout_ms=Some(0) without max_attempts must fail in 1 wire attempt.
    *fake.state.publish_error.lock().await = Some((Code::Unavailable, "transient".to_string()));
    let created = client
        .create_publisher(CreatePublisherRequest {
            endpoint: fake.addr.clone(),
            topic: "projects/test-proj/topics/zero-timeout".to_string(),
            count_threshold: Some(1),
            total_timeout_ms: Some(0),
            ..Default::default()
        })
        .await
        .expect("CreatePublisher with total_timeout_ms=0")
        .into_inner();

    // use_blocking_publisher=true is rejected as Unimplemented.
    let block_err = client
        .publish(PublishRequest {
            publisher_id: created.publisher_id.clone(),
            data: b"block".to_vec(),
            use_blocking_publisher: true,
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(block_err.code(), Code::Unimplemented);

    let publish_start = Instant::now();
    let zero_timeout_resp = client
        .publish(PublishRequest {
            publisher_id: created.publisher_id.clone(),
            data: b"zero".to_vec(),
            ..Default::default()
        })
        .await
        .expect("Publish returns in-band error")
        .into_inner();
    assert!(
        matches!(
            zero_timeout_resp.error_code,
            c if c == Code::DeadlineExceeded as i32 || c == Code::Unavailable as i32
        ),
        "expected DeadlineExceeded or Unavailable for total_timeout_ms=0, got {:?}",
        zero_timeout_resp
    );
    assert!(
        fake.state.publish_count.load(Ordering::SeqCst) <= 1,
        "total_timeout_ms=0 must not retry indefinitely (count={})",
        fake.state.publish_count.load(Ordering::SeqCst)
    );
    assert!(
        publish_start.elapsed() < Duration::from_secs(1),
        "total_timeout_ms=0 took too long: {:?}",
        publish_start.elapsed()
    );

    client
        .close_publisher(ClosePublisherRequest {
            publisher_id: created.publisher_id,
        })
        .await
        .expect("ClosePublisher");

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}

#[tokio::test]
async fn subscriber_session_invalid_transitions_and_stop_timeout() {
    let fake = fake_pubsub::start(Default::default()).await;
    let (endpoint, server_handle) = spawn_in_process_proxy().await;
    let mut client = connect_proxy_client(&endpoint).await;

    // 1. Empty command stream -> InvalidArgument.
    {
        let (cmd_tx, cmd_rx) = mpsc::channel::<SubscriberCommand>(1);
        drop(cmd_tx);
        let err = client
            .subscriber_session(ReceiverStream::new(cmd_rx))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    // 2. Non-Start first command -> InvalidArgument.
    {
        let (cmd_tx, cmd_rx) = mpsc::channel(4);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Settle(SettleMessageCommand {
                    delivery_token: "t".to_string(),
                    action: SettleAction::Ack as i32,
                })),
            })
            .await
            .unwrap();
        drop(cmd_tx);
        let err = client
            .subscriber_session(ReceiverStream::new(cmd_rx))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    // 3. Invalid Start with '/' in short subscription -> InvalidArgument.
    {
        let (cmd_tx, cmd_rx) = mpsc::channel(4);
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Start(StartSubscriberCommand {
                    endpoint: fake.addr.clone(),
                    subscription: "bad/short".to_string(),
                    project_id: "p".to_string(),
                    ..Default::default()
                })),
            })
            .await
            .unwrap();
        drop(cmd_tx);
        let err = client
            .subscriber_session(ReceiverStream::new(cmd_rx))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    // 4. Duplicate Start mid-session -> stream error.
    {
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/dup-start".to_string(),
                auto_settle_mode: AutoSettleMode::Interactive as i32,
                ..Default::default()
            },
        )
        .await;
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Start(StartSubscriberCommand {
                    endpoint: fake.addr.clone(),
                    subscription: "projects/test-proj/subscriptions/dup-start".to_string(),
                    ..Default::default()
                })),
            })
            .await
            .unwrap();
        let err = tokio::time::timeout(Duration::from_secs(2), event_stream.message())
            .await
            .expect("dup Start response")
            .unwrap_err();
        assert!(
            matches!(err.code(), Code::InvalidArgument | Code::FailedPrecondition),
            "unexpected duplicate Start status: {err:?}"
        );
    }

    // 5. StopSubscriberCommand with shutdown_timeout_ms=Some(_) -> Unimplemented.
    {
        let (cmd_tx, mut event_stream, _) = open_subscriber_session(
            &mut client,
            StartSubscriberCommand {
                endpoint: fake.addr.clone(),
                subscription: "projects/test-proj/subscriptions/stop-timeout".to_string(),
                auto_settle_mode: AutoSettleMode::Interactive as i32,
                ..Default::default()
            },
        )
        .await;
        cmd_tx
            .send(SubscriberCommand {
                command: Some(subscriber_command::Command::Stop(StopSubscriberCommand {
                    shutdown_behavior: ShutdownBehavior::WaitForProcessing as i32,
                    shutdown_timeout_ms: Some(100),
                })),
            })
            .await
            .unwrap();
        let err = tokio::time::timeout(Duration::from_secs(2), event_stream.message())
            .await
            .expect("stop timeout error")
            .unwrap_err();
        assert_eq!(err.code(), Code::Unimplemented);
    }

    client
        .shutdown_proxy(ShutdownProxyRequest {
            grace_period_ms: 1000,
        })
        .await
        .expect("ShutdownProxy");
    server_handle.await.expect("server shutdown");
}
