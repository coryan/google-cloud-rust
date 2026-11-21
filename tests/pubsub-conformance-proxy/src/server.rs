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

//! Core gRPC proxy service, capability reporting, endpoint/path resolution,
//! and error code translation.

use std::collections::HashMap;
use std::error::Error as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::Server;
use tonic::{Code as TonicCode, Request, Response, Status, Streaming};

use crate::publisher::PublisherRegistry;
use crate::pubsub::conformance::v1::pub_sub_proxy_server::{PubSubProxy, PubSubProxyServer};
use crate::pubsub::conformance::v1::{
    AwaitPublishRequest, ClientDefaults, ClosePublisherRequest, ClosePublisherResponse,
    CompressionAlgorithm, CreatePublisherRequest, CreatePublisherResponse, DrainTracesRequest,
    DrainTracesResponse, FlowControlLimitBehavior, FlushPublisherRequest, FlushPublisherResponse,
    GetCapabilitiesRequest, GetCapabilitiesResponse, PublishRequest, PublishResponse,
    ResetTracesRequest, ResetTracesResponse, ResumePublishRequest, ResumePublishResponse,
    ShutdownProxyRequest, ShutdownProxyResponse, StopPublisherRequest, StopPublisherResponse,
    SubscriberCommand, UnaryPullRequest, UnaryPullResponse,
};
use crate::subscriber::{SubscriberSessionStream, start_subscriber_session};

/// Client library version reported by [`ProxyService::get_capabilities`].
pub const CLIENT_VERSION: &str = "1.8.0";

/// Maximum gRPC message size configured for both incoming and outgoing frames
/// on [`crate::pubsub::conformance::v1::pub_sub_proxy_server::PubSubProxyServer`]
/// (64 MiB, exceeding the 32 MiB minimum required for >10 MB message tests).
pub const MAX_GRPC_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// Default grace period used on `SIGTERM`/`SIGINT` or when `grace_period_ms <= 0`.
pub const DEFAULT_SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_millis(500);

/// Coordinates graceful shutdown across the gRPC server, OS signals, and active sessions.
#[derive(Debug, Clone)]
pub struct ShutdownController {
    tx: watch::Sender<bool>,
    grace_period: Arc<Mutex<Option<Duration>>>,
}

impl Default for ShutdownController {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownController {
    /// Creates a new un-triggered [`ShutdownController`].
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(false);
        Self {
            tx,
            grace_period: Arc::new(Mutex::new(None)),
        }
    }

    /// Subscribes to global proxy shutdown notifications.
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.tx.subscribe()
    }

    /// Returns `true` if global proxy shutdown has been triggered.
    pub fn is_shutting_down(&self) -> bool {
        *self.tx.borrow()
    }

    /// Returns the configured shutdown grace period (or [`DEFAULT_SHUTDOWN_GRACE_PERIOD`]).
    pub fn grace_period(&self) -> Duration {
        self.grace_period
            .lock()
            .ok()
            .and_then(|g| *g)
            .unwrap_or(DEFAULT_SHUTDOWN_GRACE_PERIOD)
    }

    /// Triggers shutdown immediately (idempotent), defaulting grace period if unset.
    pub fn trigger_now(&self) {
        if let Ok(mut guard) = self.grace_period.lock()
            && guard.is_none()
        {
            *guard = Some(DEFAULT_SHUTDOWN_GRACE_PERIOD);
        }
        self.tx.send_if_modified(|shutting_down| {
            if *shutting_down {
                false
            } else {
                *shutting_down = true;
                true
            }
        });
    }

    /// Schedules shutdown after a brief flush delay so `ShutdownProxyResponse` is sent before `GOAWAY`.
    pub fn schedule_rpc_shutdown(&self, grace_period_ms: i64) {
        let grace = if grace_period_ms <= 0 {
            DEFAULT_SHUTDOWN_GRACE_PERIOD
        } else {
            Duration::from_millis(grace_period_ms as u64)
        };
        let first_schedule = if let Ok(mut guard) = self.grace_period.lock() {
            if guard.is_none() {
                *guard = Some(grace);
                true
            } else {
                false
            }
        } else {
            true
        };
        if first_schedule {
            let ctrl = self.clone();
            let pre_delay = grace.min(Duration::from_millis(5));
            tokio::spawn(async move {
                if !pre_delay.is_zero() {
                    tokio::time::sleep(pre_delay).await;
                }
                ctrl.trigger_now();
            });
        } else {
            self.trigger_now();
        }
    }

    /// Future that resolves once global shutdown has been triggered.
    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.subscribe();
        wait_for_watch_true(&mut rx).await;
    }
}

/// Awaits until a `watch::Receiver<bool>` observes `true` (or sleeps indefinitely if closed).
pub async fn wait_for_watch_true(rx: &mut watch::Receiver<bool>) {
    loop {
        if *rx.borrow_and_update() {
            return;
        }
        if rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// [`TcpListener`] stream adapter that sets `TCP_NODELAY` on every accepted connection.
pub struct NodelayTcpIncoming(pub TcpListener);

impl Stream for NodelayTcpIncoming {
    type Item = std::io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.0.poll_accept(cx) {
            Poll::Ready(Ok((stream, _addr))) => {
                let _ = stream.set_nodelay(true);
                Poll::Ready(Some(Ok(stream)))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Serves `service` on `listener` until `ShutdownProxy` or `os_signal`, enforcing `grace_period`.
pub async fn serve_with_grace_deadline<F>(
    listener: TcpListener,
    service: ProxyService,
    os_signal: F,
) -> Result<(), tonic::transport::Error>
where
    F: Future<Output = ()> + Send,
{
    let shutdown_ctrl = service.shutdown_controller().clone();
    let grpc_service = PubSubProxyServer::new(service)
        .max_decoding_message_size(MAX_GRPC_MESSAGE_SIZE)
        .max_encoding_message_size(MAX_GRPC_MESSAGE_SIZE);

    let server_fut = Server::builder()
        .tcp_nodelay(true)
        .add_service(grpc_service)
        .serve_with_incoming_shutdown(
            NodelayTcpIncoming(listener),
            shutdown_ctrl.wait_for_shutdown(),
        );
    tokio::pin!(server_fut);

    tokio::select! {
        res = &mut server_fut => res,
        _ = os_signal => {
            shutdown_ctrl.trigger_now();
            let grace = shutdown_ctrl.grace_period();
            match tokio::time::timeout(grace, &mut server_fut).await {
                Ok(res) => res,
                Err(_) => Ok(()),
            }
        }
        _ = shutdown_ctrl.wait_for_shutdown() => {
            let grace = shutdown_ctrl.grace_period();
            match tokio::time::timeout(grace, &mut server_fut).await {
                Ok(res) => res,
                Err(_) => Ok(()),
            }
        }
    }
}

/// Implementation of the [`PubSubProxy`] gRPC service backed by `google-cloud-pubsub`.
#[derive(Debug, Clone)]
pub struct ProxyService {
    publishers: Arc<PublisherRegistry>,
    shutdown: ShutdownController,
}

impl ProxyService {
    /// Creates a new [`ProxyService`] sharing `shutdown` for coordinated shutdown.
    pub fn new(shutdown: ShutdownController) -> Self {
        Self {
            publishers: Arc::new(PublisherRegistry::new()),
            shutdown,
        }
    }

    /// Returns the shared [`ShutdownController`] for this service.
    pub fn shutdown_controller(&self) -> &ShutdownController {
        &self.shutdown
    }
}

/// Builds the exact [`GetCapabilitiesResponse`] for `google-cloud-pubsub`.
pub fn build_capabilities_response() -> GetCapabilitiesResponse {
    GetCapabilitiesResponse {
        client_language: "rust".to_string(),
        client_library: "google-cloud-pubsub".to_string(),
        client_version: CLIENT_VERSION.to_string(),
        supports_monitoring: false,
        supports_unary_pull: false,
        supports_blocking_publish: false,
        supports_publish_flow_control: false,
        supports_publish_compression: false,
        supports_publish_hedging: true,
        supports_ordering_opt_in: false,
        supports_subscriber_ordering: false,
        supports_keepalive: true,
        supports_eod_receipt_modack: false,
        supports_auto_nack_on_drop: true,
        supports_ack_with_result: true,
        supports_custom_retry_codes: false,
        supports_exactly_once_delivery: true,
        supports_per_stream_flow_control: false,
        supports_resume_publish: true,
        supports_shutdown_behavior: true,
        supports_shutdown_timeout: false,
        defaults: Some(ClientDefaults {
            publish_batch_count_threshold: 100,
            publish_batch_byte_threshold: 1_000_000,
            publish_max_request_bytes: 10_000_000,
            publish_batch_delay_ms: 10,
            publish_total_timeout_ms: 600_000,
            publish_flow_control_max_messages: -1,
            publish_flow_control_max_bytes: -1,
            publish_flow_control_behavior: FlowControlLimitBehavior::Unspecified as i32,
            publish_compression_enabled_by_default: false,
            publish_compression_bytes_threshold: 0,
            publish_compression_algorithm: CompressionAlgorithm::Unspecified as i32,
            publish_hedging_delay_ms: 1_000,
            publish_hedging_max_tokens: 50,
            publish_hedging_refill_ratio: 0.1,
            subscriber_max_outstanding_messages: 1_000,
            subscriber_max_outstanding_bytes: 104_857_600,
            subscriber_max_extension_ms: 3_600_000,
            subscriber_min_ack_extension_ms: 0,
            subscriber_max_ack_extension_ms: 60_000,
            subscriber_adaptive_ack_extension: false,
            subscriber_per_stream_flow_control: true,
            subscriber_default_stream_ack_deadline_seconds: 60,
            keepalive_ping_interval_ms: 30_000,
            keepalive_pong_timeout_ms: 15_000,
            retry_initial_delay_ms: 100,
            retry_max_delay_ms: 60_000,
            retry_delay_multiplier: 4.0,
        }),
    }
}

/// Resolves the target gRPC endpoint from `endpoint` or `PUBSUB_EMULATOR_HOST`.
pub fn resolve_endpoint(endpoint: &str, use_emulator_env: bool) -> Result<String, Status> {
    let env_val = std::env::var("PUBSUB_EMULATOR_HOST").ok();
    resolve_endpoint_with_env(endpoint, use_emulator_env, env_val.as_deref())
}

/// Resolves the target gRPC endpoint using an explicit optional emulator host override.
pub fn resolve_endpoint_with_env(
    endpoint: &str,
    use_emulator_env: bool,
    emulator_host: Option<&str>,
) -> Result<String, Status> {
    let ep = endpoint.trim();
    let raw = if !ep.is_empty() {
        ep.to_string()
    } else if use_emulator_env {
        emulator_host
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                Status::failed_precondition(
                    "PUBSUB_EMULATOR_HOST is not set while use_emulator_env=true",
                )
            })?
    } else {
        return Err(Status::invalid_argument(
            "either endpoint or use_emulator_env=true must be specified",
        ));
    };

    if raw.contains("://") {
        Ok(raw)
    } else {
        Ok(format!("http://{raw}"))
    }
}

/// Resolves a topic or subscription path, preserving full `projects/...` names verbatim.
pub fn resolve_resource_path(
    kind: &str,
    resource: &str,
    project_id: &str,
) -> Result<String, Status> {
    if resource.is_empty() {
        return Err(Status::invalid_argument(format!(
            "{kind} resource name must not be empty"
        )));
    }
    if resource.starts_with("projects/") {
        Ok(resource.to_string())
    } else if resource.contains('/') {
        Err(Status::invalid_argument(format!(
            "short {kind} id must not contain '/': {resource}"
        )))
    } else if !project_id.is_empty() {
        Ok(format!("projects/{project_id}/{kind}/{resource}"))
    } else {
        Err(Status::invalid_argument(format!(
            "project_id is required when using a short {kind} id"
        )))
    }
}

/// Rejects unsupported client identity or custom header overrides.
pub fn check_common_overrides(
    client_id: &str,
    custom_headers: &HashMap<String, String>,
) -> Result<(), Status> {
    if !client_id.is_empty() {
        return Err(Status::unimplemented(
            "client_id override is not supported by google-cloud-pubsub",
        ));
    }
    if !custom_headers.is_empty() {
        return Err(Status::unimplemented(
            "custom_headers override is not supported by google-cloud-pubsub",
        ));
    }
    Ok(())
}

/// Converts a `google_cloud_gax::error::rpc::Code` to a canonical gRPC status integer.
pub fn gax_code_to_i32(code: google_cloud_gax::error::rpc::Code) -> i32 {
    use google_cloud_gax::error::rpc::Code;
    match code {
        Code::Ok => 0,
        Code::Cancelled => 1,
        Code::Unknown => 2,
        Code::InvalidArgument => 3,
        Code::DeadlineExceeded => 4,
        Code::NotFound => 5,
        Code::AlreadyExists => 6,
        Code::PermissionDenied => 7,
        Code::ResourceExhausted => 8,
        Code::FailedPrecondition => 9,
        Code::Aborted => 10,
        Code::OutOfRange => 11,
        Code::Unimplemented => 12,
        Code::Internal => 13,
        Code::Unavailable => 14,
        Code::DataLoss => 15,
        Code::Unauthenticated => 16,
        _ => 2,
    }
}

/// Converts a `tonic::Code` to a canonical gRPC status integer.
pub fn tonic_code_to_i32(code: TonicCode) -> i32 {
    match code {
        TonicCode::Ok => 0,
        TonicCode::Cancelled => 1,
        TonicCode::Unknown => 2,
        TonicCode::InvalidArgument => 3,
        TonicCode::DeadlineExceeded => 4,
        TonicCode::NotFound => 5,
        TonicCode::AlreadyExists => 6,
        TonicCode::PermissionDenied => 7,
        TonicCode::ResourceExhausted => 8,
        TonicCode::FailedPrecondition => 9,
        TonicCode::Aborted => 10,
        TonicCode::OutOfRange => 11,
        TonicCode::Unimplemented => 12,
        TonicCode::Internal => 13,
        TonicCode::Unavailable => 14,
        TonicCode::DataLoss => 15,
        TonicCode::Unauthenticated => 16,
    }
}

fn find_tonic_status<'a>(
    mut cur: Option<&'a (dyn std::error::Error + 'static)>,
) -> Option<&'a Status> {
    while let Some(err) = cur {
        if let Some(status) = err.downcast_ref::<Status>() {
            return Some(status);
        }
        cur = err.source();
    }
    None
}

/// Translates a `google_cloud_gax::error::Error` into `(grpc_code, message)`.
pub fn map_gax_error(err: &google_cloud_gax::error::Error) -> (i32, String) {
    if err.is_exhausted() {
        if let Some(inner) = err
            .source()
            .and_then(|s| s.downcast_ref::<google_cloud_gax::error::Error>())
        {
            return map_gax_error(inner);
        }
        if let Some(status) = find_tonic_status(err.source()) {
            return (
                tonic_code_to_i32(status.code()),
                status.message().to_string(),
            );
        }
        return (4, err.to_string());
    }

    if let Some(status) = err.status() {
        let code = gax_code_to_i32(status.code);
        let msg = if status.message.is_empty() {
            err.to_string()
        } else {
            status.message.clone()
        };
        return (code, msg);
    }

    if err.is_timeout() {
        return (4, err.to_string());
    }

    if let Some(status) = find_tonic_status(Some(err)) {
        return (
            tonic_code_to_i32(status.code()),
            status.message().to_string(),
        );
    }

    if err.is_connect() || err.is_transport() || err.is_io() {
        return (14, err.to_string());
    }

    (2, err.to_string())
}

#[tonic::async_trait]
impl PubSubProxy for ProxyService {
    async fn get_capabilities(
        &self,
        _request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        Ok(Response::new(build_capabilities_response()))
    }

    async fn reset_traces(
        &self,
        _request: Request<ResetTracesRequest>,
    ) -> Result<Response<ResetTracesResponse>, Status> {
        Err(Status::unimplemented(
            "monitoring is not supported by this proxy",
        ))
    }

    async fn drain_traces(
        &self,
        _request: Request<DrainTracesRequest>,
    ) -> Result<Response<DrainTracesResponse>, Status> {
        Err(Status::unimplemented(
            "monitoring is not supported by this proxy",
        ))
    }

    async fn create_publisher(
        &self,
        request: Request<CreatePublisherRequest>,
    ) -> Result<Response<CreatePublisherResponse>, Status> {
        self.publishers.create_publisher(request.into_inner()).await
    }

    async fn publish(
        &self,
        request: Request<PublishRequest>,
    ) -> Result<Response<PublishResponse>, Status> {
        self.publishers.publish(request.into_inner()).await
    }

    async fn await_publish(
        &self,
        request: Request<AwaitPublishRequest>,
    ) -> Result<Response<PublishResponse>, Status> {
        self.publishers.await_publish(request.into_inner()).await
    }

    async fn flush_publisher(
        &self,
        request: Request<FlushPublisherRequest>,
    ) -> Result<Response<FlushPublisherResponse>, Status> {
        self.publishers.flush_publisher(request.into_inner()).await
    }

    async fn stop_publisher(
        &self,
        request: Request<StopPublisherRequest>,
    ) -> Result<Response<StopPublisherResponse>, Status> {
        self.publishers.stop_publisher(request.into_inner()).await
    }

    async fn resume_publish(
        &self,
        request: Request<ResumePublishRequest>,
    ) -> Result<Response<ResumePublishResponse>, Status> {
        self.publishers.resume_publish(request.into_inner()).await
    }

    async fn close_publisher(
        &self,
        request: Request<ClosePublisherRequest>,
    ) -> Result<Response<ClosePublisherResponse>, Status> {
        self.publishers.close_publisher(request.into_inner()).await
    }

    type SubscriberSessionStream = SubscriberSessionStream;

    async fn subscriber_session(
        &self,
        request: Request<Streaming<SubscriberCommand>>,
    ) -> Result<Response<Self::SubscriberSessionStream>, Status> {
        start_subscriber_session(request.into_inner(), self.shutdown.clone()).await
    }

    async fn unary_pull(
        &self,
        _request: Request<UnaryPullRequest>,
    ) -> Result<Response<UnaryPullResponse>, Status> {
        Err(Status::unimplemented(
            "unary pull is not supported by google-cloud-pubsub",
        ))
    }

    async fn shutdown_proxy(
        &self,
        request: Request<ShutdownProxyRequest>,
    ) -> Result<Response<ShutdownProxyResponse>, Status> {
        let grace_ms = request.into_inner().grace_period_ms;
        self.shutdown.schedule_rpc_shutdown(grace_ms);
        Ok(Response::new(ShutdownProxyResponse {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_cloud_gax::error::Error as GaxError;
    use google_cloud_gax::error::rpc::{Code as GaxCode, Status as RpcStatus};
    use tonic::Code as TonicCode;

    #[test]
    fn capabilities_match_r2_and_library_defaults() {
        let caps = build_capabilities_response();
        assert_eq!(caps.client_language, "rust");
        assert_eq!(caps.client_library, "google-cloud-pubsub");
        assert_eq!(caps.client_version, CLIENT_VERSION);
        assert!(!caps.supports_monitoring);
        assert!(!caps.supports_unary_pull);
        assert!(!caps.supports_blocking_publish);
        assert!(!caps.supports_publish_flow_control);
        assert!(!caps.supports_publish_compression);
        assert!(caps.supports_publish_hedging);
        assert!(!caps.supports_ordering_opt_in);
        assert!(!caps.supports_subscriber_ordering);
        assert!(caps.supports_keepalive);
        assert!(!caps.supports_eod_receipt_modack);
        assert!(caps.supports_auto_nack_on_drop);
        assert!(caps.supports_ack_with_result);
        assert!(!caps.supports_custom_retry_codes);
        assert!(caps.supports_exactly_once_delivery);
        assert!(!caps.supports_per_stream_flow_control);
        assert!(caps.supports_resume_publish);
        assert!(caps.supports_shutdown_behavior);
        assert!(!caps.supports_shutdown_timeout);

        let defaults = caps.defaults.expect("defaults populated");
        assert_eq!(defaults.publish_batch_count_threshold, 100);
        assert_eq!(defaults.publish_batch_byte_threshold, 1_000_000);
        assert_eq!(defaults.publish_max_request_bytes, 10_000_000);
        assert_eq!(defaults.publish_batch_delay_ms, 10);
        assert_eq!(defaults.publish_total_timeout_ms, 600_000);
        assert_eq!(defaults.publish_flow_control_max_messages, -1);
        assert_eq!(defaults.publish_flow_control_max_bytes, -1);
        assert_eq!(defaults.subscriber_max_outstanding_bytes, 104_857_600);
        assert_eq!(defaults.subscriber_max_extension_ms, 3_600_000);
        assert_eq!(defaults.retry_delay_multiplier, 4.0);
    }

    #[test]
    fn resource_path_validation() {
        assert_eq!(
            resolve_resource_path("topics", "projects/full/topics/t1", "override").unwrap(),
            "projects/full/topics/t1"
        );
        assert_eq!(
            resolve_resource_path("subscriptions", "projects/full/subscriptions/s1", "").unwrap(),
            "projects/full/subscriptions/s1"
        );
        assert_eq!(
            resolve_resource_path("topics", "t1", "p1").unwrap(),
            "projects/p1/topics/t1"
        );
        assert_eq!(
            resolve_resource_path("subscriptions", "s1", "p1").unwrap(),
            "projects/p1/subscriptions/s1"
        );
        let err = resolve_resource_path("topics", "", "p1").unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_resource_path("topics", "bad/path", "p1").unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_resource_path("subscriptions", "bad/sub", "p1").unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_resource_path("topics", "short-topic", "").unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_resource_path("subscriptions", "short-sub", "").unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
    }

    #[test]
    fn endpoint_resolution_rules() {
        assert_eq!(
            resolve_endpoint_with_env("127.0.0.1:1234", false, None).unwrap(),
            "http://127.0.0.1:1234"
        );
        assert_eq!(
            resolve_endpoint_with_env("https://pubsub.example:443", false, None).unwrap(),
            "https://pubsub.example:443"
        );
        assert_eq!(
            resolve_endpoint_with_env("", true, Some("127.0.0.1:8085")).unwrap(),
            "http://127.0.0.1:8085"
        );
        assert_eq!(
            resolve_endpoint_with_env("", true, Some("  http://127.0.0.1:8085  ")).unwrap(),
            "http://127.0.0.1:8085"
        );
        let err = resolve_endpoint_with_env("", false, None).unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_endpoint_with_env("   ", false, None).unwrap_err();
        assert_eq!(err.code(), TonicCode::InvalidArgument);
        let err = resolve_endpoint_with_env("", true, None).unwrap_err();
        assert_eq!(err.code(), TonicCode::FailedPrecondition);
        let err = resolve_endpoint_with_env("", true, Some("")).unwrap_err();
        assert_eq!(err.code(), TonicCode::FailedPrecondition);
        let err = resolve_endpoint_with_env("", true, Some("   \t ")).unwrap_err();
        assert_eq!(err.code(), TonicCode::FailedPrecondition);
    }

    #[test]
    fn common_overrides_rejected_as_unimplemented() {
        let err = check_common_overrides("custom-id", &HashMap::new()).unwrap_err();
        assert_eq!(err.code(), TonicCode::Unimplemented);

        let mut headers = HashMap::new();
        headers.insert("x-test".to_string(), "1".to_string());
        let err = check_common_overrides("", &headers).unwrap_err();
        assert_eq!(err.code(), TonicCode::Unimplemented);
    }

    #[test]
    fn map_gax_error_covers_service_exhausted_timeout_and_connect() {
        let rpc_err = GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::DataLoss)
                .set_message("bad checksum"),
        );
        assert_eq!(map_gax_error(&rpc_err), (15, "bad checksum".to_string()));

        let empty_msg_rpc = GaxError::service(RpcStatus::default().set_code(GaxCode::Aborted));
        let (code, msg) = map_gax_error(&empty_msg_rpc);
        assert_eq!(code, 10);
        assert!(!msg.is_empty());

        let exhausted = GaxError::exhausted(rpc_err);
        assert_eq!(map_gax_error(&exhausted), (15, "bad checksum".to_string()));

        let exhausted_tonic = GaxError::exhausted(Status::resource_exhausted("quota hit"));
        assert_eq!(
            map_gax_error(&exhausted_tonic),
            (8, "quota hit".to_string())
        );

        let timeout_err = GaxError::timeout(Status::deadline_exceeded("timed out"));
        assert_eq!(map_gax_error(&timeout_err).0, 4);

        let connect_err = GaxError::connect("connection refused");
        assert_eq!(map_gax_error(&connect_err).0, 14);

        let io_err = GaxError::io("broken pipe");
        assert_eq!(map_gax_error(&io_err).0, 14);

        let ser_err = GaxError::ser("bad payload");
        assert_eq!(map_gax_error(&ser_err).0, 2);
    }

    #[tokio::test]
    async fn shutdown_controller_tracks_grace_period_and_triggers() {
        let ctrl = ShutdownController::new();
        assert!(!ctrl.is_shutting_down());
        assert_eq!(ctrl.grace_period(), DEFAULT_SHUTDOWN_GRACE_PERIOD);

        ctrl.schedule_rpc_shutdown(125);
        assert_eq!(ctrl.grace_period(), Duration::from_millis(125));
        ctrl.wait_for_shutdown().await;
        assert!(ctrl.is_shutting_down());

        let ctrl_default = ShutdownController::new();
        ctrl_default.schedule_rpc_shutdown(0);
        assert_eq!(ctrl_default.grace_period(), DEFAULT_SHUTDOWN_GRACE_PERIOD);
        ctrl_default.wait_for_shutdown().await;
        assert!(ctrl_default.is_shutting_down());
    }
}
