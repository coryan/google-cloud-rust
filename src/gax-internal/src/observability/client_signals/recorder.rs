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

//! Capture telemetry information for a typical client request.
//!
//! In this module we use the nomenclature from go/clo:product-requirements-v1
//!
//! We want the client library to emit telemetry signals ( spans, duration metrics, and logs) for
//! each client (T3) and low-level request (T4). To meet the requirements we need to capture
//! information as the request makes progress. For example, the client request telemetry includes
//! information about the last low-level request, such as the remote server IP address and port. It
//! is difficult to carry this information through the different layers without breaking changes
//! APIs.
//!
//! This module solves that problem by setting a task-local (think "thread local" but for
//! asynchronous tasks) variable valid for the full request. Each layer adds information to this
//! variable. Once the telemetry layer is ready to emit a signal it consults the variable and uses
//! the latest snapshot to populate the attributes of the signal.
//!
//! # Example
//! ```
//! # use google_cloud_gax_internal::observability::RequestRecorder;
//! use google_cloud_gax_internal::observability::DurationMetric;
//! use google_cloud_gax_internal::options::InstrumentationClientInfo;
//! async fn telemetry_layer() -> google_cloud_gax::Result<String> {
//!     let t3_span = tracing::info_span!("client_request" /* more attributes */);
//!     let recorder = RequestRecorder::new(info());
//!     // Calls `transport_layer()` and capture all the T3 operations,
//!     // including the duration metric, spans, and logs.
//!     recorder.t3_scope(t3_metric(), t3_span, transport_layer()).await
//! }
//!
//! fn t3_metric() -> DurationMetric {
//! # panic!();
//! }
//! fn info() -> InstrumentationClientInfo {
//! # panic!();
//! }
//! async fn transport_layer() -> google_cloud_gax::Result<String> {
//! # panic!("")
//! }
//! ```

use crate::observability::client_signals::with_client_signals::WithRecorder;
use crate::observability::{ClientSignalsExt, DurationMetric, RequestStart};
use crate::options::InstrumentationClientInfo;
#[cfg(feature = "_internal-http-client")]
use google_cloud_gax::error::Error;
use google_cloud_gax::options::RequestOptions;
use google_cloud_gax::options::internal::{PathTemplate, RequestOptionsExt};
use reqwest::Method;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;
use tracing::Span;
use tracing::instrument::Instrumented;

tokio::task_local! {
    static RECORDER: RequestRecorder;
}

/// Collects key information about a request to update the telemetry information.
///
/// The name should evoke a "flight recorder" the devices to records interesting events about airplane operations.
#[derive(Clone, Debug)]
pub struct RequestRecorder {
    inner: Arc<Mutex<T3Snapshot>>,
}

impl RequestRecorder {
    /// Creates a new request recorder based on the client library instrumentation in `info`.
    pub fn new(info: InstrumentationClientInfo) -> Self {
        let inner = T3Snapshot::new(info);
        let inner = Arc::new(Mutex::new(inner));
        Self { inner }
    }

    /// Runs a `future` in the scope of a request recorder.
    ///
    /// # Example
    /// See the [module reference][recorder].
    pub fn t3_scope<F, R>(
        self,
        metric: DurationMetric,
        span: Span,
        future: F,
    ) -> tokio::task::futures::TaskLocalFuture<Self, WithRecorder<Instrumented<F>>>
    where
        F: std::future::Future<Output = Result<R, Error>>,
    {
        let wrapped = future.with_recorder(self.clone(), metric, span);
        RECORDER.scope(self, wrapped)
    }

    pub fn current() -> Option<Self> {
        RECORDER.try_get().ok()
    }

    pub fn t3_snapshot(&self) -> T3Snapshot {
        let guard = self.inner.lock().expect("never poisoned");
        guard.clone()
    }

    pub fn extend(&self, options: RequestOptions) -> RequestOptions {
        use google_cloud_gax::options::internal::RequestOptionsExt;
        options.insert_extension(self.clone())
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_request(&self, options: &RequestOptions, request: &reqwest::Request) {
        let mut guard = self.inner.lock().expect("never poisoned");
        let snapshot = T4Snapshot {
            start: Instant::now(),
            server_address: None,
            url_template: options.get_extension::<PathTemplate>().map(|e| e.0),
            rpc_method: None,
            http_method: Some(request.method().clone()),
            url: Some(request.url().to_string()),
        };
        guard.t4_snapshot = Some(snapshot);
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_response(&self, response: &reqwest::Response) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.attempt_count += 1;
        if let Some(s) = guard.t4_snapshot.as_mut() {
            s.server_address = response.remote_addr();
        }
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_error(&self, _err: &Error) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.attempt_count += 1;
        if let Some(s) = guard.t4_snapshot.as_mut() {
            s.server_address = None;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct T3Snapshot {
    pub start: Instant,
    pub info: InstrumentationClientInfo,
    pub attempt_count: u32,
    pub t4_snapshot: Option<T4Snapshot>,
}

impl T3Snapshot {
    fn new(info: InstrumentationClientInfo) -> Self {
        let start = Instant::now();
        Self {
            start,
            info,
            attempt_count: 0_u32,
            t4_snapshot: None,
        }
    }

    // TODO(#48..) - remove once it is no longer used.
    pub fn t3_start(&self) -> RequestStart {
        let t4 = self.t4_snapshot.as_ref();
        RequestStart::from_parts(
            self.start,
            self.info.clone(),
            t4.and_then(|s| s.url_template.clone()).unwrap_or_default(),
            t4.and_then(|s| s.rpc_method.clone()).unwrap_or_default(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct T4Snapshot {
    pub start: Instant,
    pub server_address: Option<SocketAddr>,
    pub url_template: Option<&'static str>,
    pub rpc_method: Option<&'static str>,
    pub http_method: Option<Method>,
    pub url: Option<String>,
}
