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

//! Publisher lifecycle, message validation, ordering, hedging, and retry translation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use google_cloud_auth::credentials::anonymous::Builder as AnonymousCredentials;
use google_cloud_gax::exponential_backoff::ExponentialBackoffBuilder;
use google_cloud_gax::retry_policy::RetryPolicyExt;
use google_cloud_pubsub::client::Publisher;
use google_cloud_pubsub::error::PublishError;
use google_cloud_pubsub::model::Message;
use google_cloud_pubsub::publisher::{HedgingOptions, PublishFuture};
use google_cloud_pubsub::retry_policy::RetryableErrors;
use tokio::sync::watch;
use tonic::{Response, Status};

use crate::pubsub::conformance::v1::{
    AwaitPublishRequest, ClosePublisherRequest, ClosePublisherResponse, CreatePublisherRequest,
    CreatePublisherResponse, FlushPublisherRequest, FlushPublisherResponse, PublishRequest,
    PublishResponse, ResumePublishRequest, ResumePublishResponse, StopPublisherRequest,
    StopPublisherResponse,
};
use crate::server::{
    check_common_overrides, map_gax_error, resolve_endpoint, resolve_resource_path,
    wait_for_watch_true,
};

struct DrainGuard(Option<watch::Sender<bool>>);

impl Drop for DrainGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send_replace(true);
        }
    }
}

struct PublisherState {
    publisher: Option<Publisher>,
    stopped: bool,
    stop_drain: Option<watch::Receiver<bool>>,
    next_token: u64,
    futures: HashMap<String, PublishFuture>,
}

struct PublisherEntry {
    state: Mutex<PublisherState>,
}

/// Manages active [`Publisher`] instances and deferred `async_fire` publish futures.
#[derive(Default)]
pub struct PublisherRegistry {
    next_id: AtomicU64,
    publishers: Mutex<HashMap<String, Arc<PublisherEntry>>>,
}

impl std::fmt::Debug for PublisherRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublisherRegistry")
            .field("next_id", &self.next_id)
            .finish_non_exhaustive()
    }
}

impl PublisherRegistry {
    /// Creates an empty [`PublisherRegistry`].
    pub fn new() -> Self {
        Self::default()
    }

    fn get_entry(&self, publisher_id: &str) -> Result<Arc<PublisherEntry>, Status> {
        let guard = self
            .publishers
            .lock()
            .map_err(|_| Status::internal("publisher registry mutex poisoned"))?;
        guard
            .get(publisher_id)
            .cloned()
            .ok_or_else(|| Status::not_found(format!("unknown publisher_id: {publisher_id}")))
    }

    /// Validates configuration and constructs a configured [`Publisher`].
    pub async fn create_publisher(
        &self,
        req: CreatePublisherRequest,
    ) -> Result<Response<CreatePublisherResponse>, Status> {
        check_common_overrides(&req.client_id, &req.custom_headers)?;

        if req.max_outstanding_messages.is_some()
            || req.max_outstanding_bytes.is_some()
            || req.limit_exceeded_behavior != 0
        {
            return Err(Status::unimplemented(
                "publish flow control is not supported by google-cloud-pubsub",
            ));
        }
        if req.enable_compression == Some(true)
            || req.compression_bytes_threshold.is_some()
            || req.compression_algorithm != 0
        {
            return Err(Status::unimplemented(
                "publish compression is not supported by google-cloud-pubsub",
            ));
        }
        if !req.retry_codes.is_empty() {
            return Err(Status::unimplemented(
                "custom retry_codes are not supported by google-cloud-pubsub",
            ));
        }

        let endpoint = resolve_endpoint(&req.endpoint, req.use_emulator_env)?;
        let topic = resolve_resource_path("topics", &req.topic, &req.project_id)?;

        // `enable_monitoring`, `enable_message_ordering`, and `num_workers` are
        // accepted without error: monitoring is handled out of scope (R2),
        // ordering is inferred automatically from non-empty ordering_key, and
        // worker count is fixed internally.
        let _ = (
            req.enable_monitoring,
            req.enable_message_ordering,
            req.num_workers,
        );

        let mut builder = Publisher::builder(topic)
            .with_endpoint(endpoint)
            .with_credentials(AnonymousCredentials::new().build());

        if let Some(count) = req.count_threshold {
            let clamped = u32::try_from(count.clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX);
            builder = builder.set_message_count_threshold(clamped);
        }
        if let Some(bytes) = req.byte_threshold {
            let clamped = u32::try_from(bytes.clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX);
            builder = builder.set_byte_threshold(clamped);
        }
        if let Some(delay_ms) = req.delay_threshold_ms {
            let ms = u64::try_from(delay_ms.max(0)).unwrap_or(0);
            builder = builder.set_delay_threshold(Duration::from_millis(ms));
        }

        if let Some(hedging) = req.hedging {
            if hedging.enabled {
                let mut opts = HedgingOptions::new();
                if let Some(delay_ms) = hedging.delay_ms {
                    if delay_ms < 0 {
                        return Err(Status::invalid_argument(
                            "hedging delay_ms must be non-negative",
                        ));
                    }
                    opts = opts.set_delay(Duration::from_millis(delay_ms as u64));
                }
                if let Some(max_tokens) = hedging.max_tokens {
                    if max_tokens < 0 {
                        return Err(Status::invalid_argument(
                            "hedging max_tokens must be non-negative",
                        ));
                    }
                    let tokens = u32::try_from(max_tokens).unwrap_or(u32::MAX);
                    opts = opts.set_max_tokens(tokens);
                }
                if let Some(refill_ratio) = hedging.refill_ratio {
                    if !refill_ratio.is_finite() || refill_ratio < 0.0 {
                        return Err(Status::invalid_argument(
                            "hedging refill_ratio must be finite and non-negative",
                        ));
                    }
                    opts = opts.set_refill_ratio(refill_ratio as f32);
                }
                builder = builder.set_hedging_options(opts);
            } else {
                builder = builder.set_or_clear_hedging_options(None);
            }
        }

        if let Some(ms) = req.total_timeout_ms
            && ms < 0
        {
            return Err(Status::invalid_argument(
                "total_timeout_ms must be non-negative",
            ));
        }
        if let Some(att) = req.max_attempts
            && att < 0
        {
            return Err(Status::invalid_argument(
                "max_attempts must be non-negative",
            ));
        }
        if req.total_timeout_ms.is_some() || req.max_attempts.is_some() {
            let base = RetryableErrors;
            match (req.total_timeout_ms, req.max_attempts) {
                (Some(ms), Some(att)) => {
                    let attempts = u32::try_from(att).unwrap_or(u32::MAX);
                    builder = builder.with_retry_policy(
                        base.with_time_limit(Duration::from_millis(ms as u64))
                            .with_attempt_limit(attempts),
                    );
                }
                (Some(ms), None) => {
                    builder = builder
                        .with_retry_policy(base.with_time_limit(Duration::from_millis(ms as u64)));
                }
                (None, Some(att)) => {
                    let attempts = u32::try_from(att).unwrap_or(u32::MAX);
                    builder = builder.with_retry_policy(
                        base.with_time_limit(Duration::from_secs(600))
                            .with_attempt_limit(attempts),
                    );
                }
                (None, None) => unreachable!(),
            }
        }

        if req.initial_retry_delay_ms.is_some()
            || req.max_retry_delay_ms.is_some()
            || req.retry_delay_multiplier.is_some()
        {
            if let Some(init) = req.initial_retry_delay_ms
                && init <= 0
            {
                return Err(Status::invalid_argument(
                    "initial_retry_delay_ms must be positive",
                ));
            }
            if let Some(max) = req.max_retry_delay_ms
                && max <= 0
            {
                return Err(Status::invalid_argument(
                    "max_retry_delay_ms must be positive",
                ));
            }
            if let Some(scaling) = req.retry_delay_multiplier
                && (!scaling.is_finite() || scaling < 1.0)
            {
                return Err(Status::invalid_argument(
                    "retry_delay_multiplier must be finite and >= 1.0",
                ));
            }

            let max_ms = req.max_retry_delay_ms.unwrap_or(60_000) as u64;
            let init_ms = match req.initial_retry_delay_ms {
                Some(v) => v as u64,
                None => 100_u64.min(max_ms),
            };
            let scaling = req.retry_delay_multiplier.unwrap_or(4.0);
            let backoff = ExponentialBackoffBuilder::new()
                .with_initial_delay(Duration::from_millis(init_ms))
                .with_maximum_delay(Duration::from_millis(max_ms))
                .with_scaling(scaling)
                .build()
                .map_err(|e| Status::invalid_argument(format!("invalid backoff policy: {e}")))?;
            builder = builder.with_backoff_policy(backoff);
        }

        let publisher = builder
            .build()
            .await
            .map_err(|e| Status::internal(format!("failed to build publisher: {e}")))?;

        let id = format!("pub-{}", self.next_id.fetch_add(1, Ordering::Relaxed) + 1);
        let entry = Arc::new(PublisherEntry {
            state: Mutex::new(PublisherState {
                publisher: Some(publisher),
                stopped: false,
                stop_drain: None,
                next_token: 1,
                futures: HashMap::new(),
            }),
        });

        let mut guard = self
            .publishers
            .lock()
            .map_err(|_| Status::internal("publisher registry mutex poisoned"))?;
        guard.insert(id.clone(), entry);

        Ok(Response::new(CreatePublisherResponse { publisher_id: id }))
    }

    /// Publishes a single message, synchronously resolving or returning an `async_fire` token.
    pub async fn publish(&self, req: PublishRequest) -> Result<Response<PublishResponse>, Status> {
        if req.use_blocking_publisher {
            return Err(Status::unimplemented(
                "BlockingPublisher is not supported by google-cloud-pubsub",
            ));
        }

        let entry = self.get_entry(&req.publisher_id)?;

        let maybe_fut: Result<(Option<PublishFuture>, Option<String>, bool), PublishResponse> = {
            let mut guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;

            if guard.stopped || guard.publisher.is_none() {
                Err(PublishResponse {
                    message_id: String::new(),
                    error_code: 9,
                    error_message: "publisher has been stopped".to_string(),
                    publish_token: String::new(),
                    caller_attributes_mutated: false,
                })
            } else {
                match prepare_message(&req) {
                    Err(err_resp) => Err(err_resp),
                    Ok((msg, caller_attributes_mutated)) => {
                        let pub_ref = guard
                            .publisher
                            .as_ref()
                            .expect("checked publisher is Some above");
                        let fut = pub_ref.publish(msg);
                        if req.async_fire {
                            let token = format!("tok-{}", guard.next_token);
                            guard.next_token += 1;
                            guard.futures.insert(token.clone(), fut);
                            Ok((None, Some(token), caller_attributes_mutated))
                        } else {
                            Ok((Some(fut), None, caller_attributes_mutated))
                        }
                    }
                }
            }
        };

        match maybe_fut {
            Err(resp) => Ok(Response::new(resp)),
            Ok((None, Some(token), caller_attributes_mutated)) => {
                Ok(Response::new(PublishResponse {
                    message_id: String::new(),
                    error_code: 0,
                    error_message: String::new(),
                    publish_token: token,
                    caller_attributes_mutated,
                }))
            }
            Ok((Some(fut), None, caller_attributes_mutated)) => {
                let result = fut.await;
                Ok(Response::new(map_publish_result(
                    result,
                    caller_attributes_mutated,
                )))
            }
            Ok(_) => unreachable!("invalid publish dispatch state"),
        }
    }

    /// Awaits a previously enqueued `async_fire` publish future.
    pub async fn await_publish(
        &self,
        req: AwaitPublishRequest,
    ) -> Result<Response<PublishResponse>, Status> {
        let entry = self.get_entry(&req.publisher_id)?;
        let fut = {
            let mut guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;
            guard.futures.remove(&req.publish_token).ok_or_else(|| {
                Status::not_found(format!("unknown publish_token: {}", req.publish_token))
            })?
        };
        let result = fut.await;
        Ok(Response::new(map_publish_result(result, false)))
    }

    /// Flushes all pending batches for `publisher_id`.
    pub async fn flush_publisher(
        &self,
        req: FlushPublisherRequest,
    ) -> Result<Response<FlushPublisherResponse>, Status> {
        let entry = self.get_entry(&req.publisher_id)?;
        let (publisher, drain_rx) = {
            let guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;
            let pub_opt = guard.publisher.clone();
            let drain_opt = if pub_opt.is_none() {
                guard.stop_drain.clone()
            } else {
                None
            };
            (pub_opt, drain_opt)
        };
        if let Some(p) = publisher {
            p.flush().await;
        } else if let Some(mut rx) = drain_rx {
            wait_for_watch_true(&mut rx).await;
        }
        Ok(Response::new(FlushPublisherResponse {}))
    }

    /// Stops `publisher_id`, flushing in-flight publishes while retaining pending `async_fire` futures.
    pub async fn stop_publisher(
        &self,
        req: StopPublisherRequest,
    ) -> Result<Response<StopPublisherResponse>, Status> {
        let entry = self.get_entry(&req.publisher_id)?;
        let (publisher, drain_guard, drain_rx) = {
            let mut guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;
            guard.stopped = true;
            if let Some(p) = guard.publisher.take() {
                let (drain_tx, drain_rx) = watch::channel(false);
                guard.stop_drain = Some(drain_rx);
                (Some(p), Some(DrainGuard(Some(drain_tx))), None)
            } else {
                (None, None, guard.stop_drain.clone())
            }
        };
        if let Some(p) = publisher {
            p.flush().await;
            drop(drain_guard);
        } else if let Some(mut rx) = drain_rx {
            wait_for_watch_true(&mut rx).await;
        }
        Ok(Response::new(StopPublisherResponse {}))
    }

    /// Resumes publishing on a paused ordering key.
    pub async fn resume_publish(
        &self,
        req: ResumePublishRequest,
    ) -> Result<Response<ResumePublishResponse>, Status> {
        let entry = self.get_entry(&req.publisher_id)?;
        let publisher = {
            let guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;
            if guard.stopped {
                return Err(Status::failed_precondition("publisher has been stopped"));
            }
            guard.publisher.clone()
        };
        if let Some(p) = publisher {
            p.resume_publish(req.ordering_key);
        }
        Ok(Response::new(ResumePublishResponse {}))
    }

    /// Releases all resources associated with `publisher_id`.
    pub async fn close_publisher(
        &self,
        req: ClosePublisherRequest,
    ) -> Result<Response<ClosePublisherResponse>, Status> {
        let entry = {
            let mut guard = self
                .publishers
                .lock()
                .map_err(|_| Status::internal("publisher registry mutex poisoned"))?;
            guard.remove(&req.publisher_id).ok_or_else(|| {
                Status::not_found(format!("unknown publisher_id: {}", req.publisher_id))
            })?
        };
        let publisher = {
            let mut guard = entry
                .state
                .lock()
                .map_err(|_| Status::internal("publisher state mutex poisoned"))?;
            guard.stopped = true;
            guard.futures.clear();
            guard.publisher.take()
        };
        drop(publisher);
        Ok(Response::new(ClosePublisherResponse {}))
    }
}

/// Validates attributes (including `raw_attributes` UTF-8 checks) and constructs [`Message`].
pub fn prepare_message(req: &PublishRequest) -> Result<(Message, bool), PublishResponse> {
    let mut attrs = req.attributes.clone();
    for raw in &req.raw_attributes {
        let key = match std::str::from_utf8(&raw.key) {
            Ok(s) => s.to_string(),
            Err(e) => {
                return Err(PublishResponse {
                    message_id: String::new(),
                    error_code: 3,
                    error_message: format!("invalid non-UTF-8 attribute key: {e}"),
                    publish_token: String::new(),
                    caller_attributes_mutated: false,
                });
            }
        };
        let value = match std::str::from_utf8(&raw.value) {
            Ok(s) => s.to_string(),
            Err(e) => {
                return Err(PublishResponse {
                    message_id: String::new(),
                    error_code: 3,
                    error_message: format!("invalid non-UTF-8 attribute value: {e}"),
                    publish_token: String::new(),
                    caller_attributes_mutated: false,
                });
            }
        };
        attrs.insert(key, value);
    }

    let original_attrs = if req.verify_attributes_immutability {
        Some(attrs.clone())
    } else {
        None
    };

    let mut msg = Message::new()
        .set_data(req.data.clone())
        .set_attributes(attrs.clone());
    if !req.ordering_key.is_empty() {
        msg = msg.set_ordering_key(req.ordering_key.clone());
    }

    let caller_attributes_mutated = original_attrs.is_some_and(|orig| orig != attrs);
    Ok((msg, caller_attributes_mutated))
}

/// Maps a [`PublishFuture`] output into a [`PublishResponse`].
pub fn map_publish_result(
    result: Result<String, PublishError>,
    caller_attributes_mutated: bool,
) -> PublishResponse {
    match result {
        Ok(message_id) => PublishResponse {
            message_id,
            error_code: 0,
            error_message: String::new(),
            publish_token: String::new(),
            caller_attributes_mutated,
        },
        Err(err) => {
            let (error_code, error_message) = map_publish_error(&err);
            PublishResponse {
                message_id: String::new(),
                error_code,
                error_message,
                publish_token: String::new(),
                caller_attributes_mutated,
            }
        }
    }
}

/// Translates a [`PublishError`] into `(grpc_code, message)`.
pub fn map_publish_error(err: &PublishError) -> (i32, String) {
    match err {
        PublishError::Rpc(source) => map_gax_error(source.as_ref()),
        PublishError::OrderingKeyPaused => (9, err.to_string()),
        PublishError::Shutdown => (9, err.to_string()),
        _ => (2, err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubsub::conformance::v1::RawAttribute;

    #[test]
    fn non_utf8_raw_attributes_rejected_client_side() {
        let req = PublishRequest {
            publisher_id: "pub-1".to_string(),
            data: b"hello".to_vec(),
            raw_attributes: vec![RawAttribute {
                key: b"valid".to_vec(),
                value: vec![0xff, 0xfe],
            }],
            ..Default::default()
        };
        let err = prepare_message(&req).unwrap_err();
        assert_eq!(err.error_code, 3);
        assert!(err.error_message.contains("non-UTF-8"));
    }

    #[test]
    fn non_utf8_raw_attribute_key_rejected_client_side() {
        let req = PublishRequest {
            publisher_id: "pub-1".to_string(),
            data: b"hello".to_vec(),
            raw_attributes: vec![RawAttribute {
                key: vec![0x80],
                value: b"ok".to_vec(),
            }],
            ..Default::default()
        };
        let err = prepare_message(&req).unwrap_err();
        assert_eq!(err.error_code, 3);
    }

    #[test]
    fn attributes_immutability_verified() {
        let mut attrs = HashMap::new();
        attrs.insert("k".to_string(), "v".to_string());
        let req = PublishRequest {
            publisher_id: "pub-1".to_string(),
            data: b"payload".to_vec(),
            attributes: attrs,
            verify_attributes_immutability: true,
            ..Default::default()
        };
        let (msg, mutated) = prepare_message(&req).unwrap();
        assert!(!mutated);
        assert_eq!(msg.attributes.get("k").map(String::as_str), Some("v"));
    }

    #[tokio::test]
    async fn stop_publisher_marks_stopped_and_rejects_future_publishes_without_wire_rpc() {
        let reg = PublisherRegistry::new();
        let created = reg
            .create_publisher(CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/test/topics/t1".to_string(),
                ..Default::default()
            })
            .await
            .expect("create publisher")
            .into_inner();

        reg.stop_publisher(StopPublisherRequest {
            publisher_id: created.publisher_id.clone(),
        })
        .await
        .expect("stop publisher");

        let resp = reg
            .publish(PublishRequest {
                publisher_id: created.publisher_id.clone(),
                data: b"after-stop".to_vec(),
                ..Default::default()
            })
            .await
            .expect("publish returns error response")
            .into_inner();
        assert_eq!(resp.error_code, 9);
        assert!(resp.error_message.contains("stopped"));

        let resume_err = reg
            .resume_publish(ResumePublishRequest {
                publisher_id: created.publisher_id.clone(),
                ordering_key: "k1".to_string(),
            })
            .await
            .unwrap_err();
        assert_eq!(resume_err.code(), tonic::Code::FailedPrecondition);

        reg.close_publisher(ClosePublisherRequest {
            publisher_id: created.publisher_id,
        })
        .await
        .expect("close publisher");
    }

    #[tokio::test]
    async fn unsupported_create_publisher_knobs_return_unimplemented() {
        let reg = PublisherRegistry::new();

        for req in [
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                max_outstanding_messages: Some(10),
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                max_outstanding_bytes: Some(1024),
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                limit_exceeded_behavior: 2,
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                enable_compression: Some(true),
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                compression_bytes_threshold: Some(256),
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                compression_algorithm: 1,
                ..Default::default()
            },
            CreatePublisherRequest {
                endpoint: "127.0.0.1:1".to_string(),
                topic: "projects/p/topics/t".to_string(),
                retry_codes: vec![14],
                ..Default::default()
            },
        ] {
            let err = reg.create_publisher(req).await.unwrap_err();
            assert_eq!(err.code(), tonic::Code::Unimplemented);
        }
    }

    #[tokio::test]
    async fn invalid_retry_backoff_and_hedging_knobs_rejected() {
        use crate::pubsub::conformance::v1::HedgingSettings;
        let reg = PublisherRegistry::new();
        let base = CreatePublisherRequest {
            endpoint: "127.0.0.1:1".to_string(),
            topic: "projects/p/topics/t".to_string(),
            ..Default::default()
        };

        for bad in [
            CreatePublisherRequest {
                total_timeout_ms: Some(-1),
                ..base.clone()
            },
            CreatePublisherRequest {
                max_attempts: Some(-1),
                ..base.clone()
            },
            CreatePublisherRequest {
                initial_retry_delay_ms: Some(0),
                ..base.clone()
            },
            CreatePublisherRequest {
                initial_retry_delay_ms: Some(-5),
                ..base.clone()
            },
            CreatePublisherRequest {
                max_retry_delay_ms: Some(0),
                ..base.clone()
            },
            CreatePublisherRequest {
                max_retry_delay_ms: Some(-10),
                ..base.clone()
            },
            CreatePublisherRequest {
                initial_retry_delay_ms: Some(200),
                max_retry_delay_ms: Some(100),
                ..base.clone()
            },
            CreatePublisherRequest {
                retry_delay_multiplier: Some(0.5),
                ..base.clone()
            },
            CreatePublisherRequest {
                retry_delay_multiplier: Some(f64::NAN),
                ..base.clone()
            },
            CreatePublisherRequest {
                retry_delay_multiplier: Some(f64::INFINITY),
                ..base.clone()
            },
            CreatePublisherRequest {
                retry_delay_multiplier: Some(f64::NEG_INFINITY),
                ..base.clone()
            },
            CreatePublisherRequest {
                hedging: Some(HedgingSettings {
                    enabled: true,
                    delay_ms: Some(-1),
                    ..Default::default()
                }),
                ..base.clone()
            },
            CreatePublisherRequest {
                hedging: Some(HedgingSettings {
                    enabled: true,
                    max_tokens: Some(-1),
                    ..Default::default()
                }),
                ..base.clone()
            },
            CreatePublisherRequest {
                hedging: Some(HedgingSettings {
                    enabled: true,
                    refill_ratio: Some(-0.1),
                    ..Default::default()
                }),
                ..base.clone()
            },
            CreatePublisherRequest {
                hedging: Some(HedgingSettings {
                    enabled: true,
                    refill_ratio: Some(f64::NAN),
                    ..Default::default()
                }),
                ..base.clone()
            },
            CreatePublisherRequest {
                hedging: Some(HedgingSettings {
                    enabled: true,
                    refill_ratio: Some(f64::INFINITY),
                    ..Default::default()
                }),
                ..base.clone()
            },
        ] {
            let err = reg.create_publisher(bad).await.unwrap_err();
            assert_eq!(err.code(), tonic::Code::InvalidArgument);
        }

        // Valid boundary cases: total_timeout_ms=0, max_attempts=0, max_retry_delay_ms<100 with unset initial.
        for ok_req in [
            CreatePublisherRequest {
                total_timeout_ms: Some(0),
                ..base.clone()
            },
            CreatePublisherRequest {
                max_attempts: Some(0),
                ..base.clone()
            },
            CreatePublisherRequest {
                max_retry_delay_ms: Some(50),
                retry_delay_multiplier: Some(1.0),
                ..base
            },
        ] {
            let created = reg
                .create_publisher(ok_req)
                .await
                .expect("valid config")
                .into_inner();
            reg.close_publisher(ClosePublisherRequest {
                publisher_id: created.publisher_id,
            })
            .await
            .expect("close");
        }
    }

    #[tokio::test]
    async fn publisher_not_found_blocking_and_error_mappings() {
        use google_cloud_gax::error::Error as GaxError;
        use google_cloud_gax::error::rpc::{Code as GaxCode, Status as RpcStatus};

        assert_eq!(map_publish_error(&PublishError::Shutdown).0, 9);
        assert_eq!(map_publish_error(&PublishError::OrderingKeyPaused).0, 9);
        let rpc_pub_err = PublishError::Rpc(std::sync::Arc::new(GaxError::service(
            RpcStatus::default()
                .set_code(GaxCode::PermissionDenied)
                .set_message("denied"),
        )));
        assert_eq!(map_publish_error(&rpc_pub_err), (7, "denied".to_string()));

        let reg = PublisherRegistry::new();
        let err = reg
            .publish(PublishRequest {
                publisher_id: "pub-1".to_string(),
                use_blocking_publisher: true,
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unimplemented);

        assert_eq!(
            reg.publish(PublishRequest {
                publisher_id: "missing".to_string(),
                ..Default::default()
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            reg.await_publish(AwaitPublishRequest {
                publisher_id: "missing".to_string(),
                publish_token: "tok-1".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            reg.flush_publisher(FlushPublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            reg.stop_publisher(StopPublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            reg.resume_publish(ResumePublishRequest {
                publisher_id: "missing".to_string(),
                ordering_key: "k1".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            reg.close_publisher(ClosePublisherRequest {
                publisher_id: "missing".to_string(),
            })
            .await
            .unwrap_err()
            .code(),
            tonic::Code::NotFound
        );
    }
}
