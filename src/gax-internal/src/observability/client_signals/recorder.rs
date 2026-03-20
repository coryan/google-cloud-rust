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

use crate::observability::attributes::RPC_SYSTEM_HTTP;
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

/// Capture telemetry information for a typical client request.
///
/// In this type we use the nomenclature from go/clo:product-requirements-v1
///
/// We want the client library to emit telemetry signals ( spans, duration metrics, and logs) for
/// each client (T3) and low-level request (T4). To meet the requirements we need to capture
/// information as the request makes progress. For example, the client request telemetry includes
/// information about the last low-level request, such as the remote server IP address and port. It
/// is difficult to carry this information through the different layers without breaking changes
/// APIs.
///
/// This type solves that problem by setting a task-local (think "thread local" but for
/// asynchronous tasks) variable valid for the full request. Each layer adds information to this
/// variable. Once the telemetry layer is ready to emit a signal it consults the variable and uses
/// the latest snapshot to populate the attributes of the signal.
///
/// # Example
/// ```
/// # use google_cloud_gax_internal::observability::RequestRecorder;
/// use google_cloud_gax_internal::observability::DurationMetric;
/// use google_cloud_gax_internal::options::InstrumentationClientInfo;
/// async fn telemetry_layer() -> google_cloud_gax::Result<String> {
///     let t3_span = tracing::info_span!("client_request" /* more attributes */);
///     let recorder = RequestRecorder::new(info());
///     // Calls `transport_layer()` and capture all the T3 operations,
///     // including the duration metric, spans, and logs.
///     recorder.t3_scope(t3_metric(), t3_span, transport_layer()).await
/// }
///
/// fn t3_metric() -> DurationMetric {
/// # panic!();
/// }
/// fn info() -> InstrumentationClientInfo {
/// # panic!();
/// }
/// async fn transport_layer() -> google_cloud_gax::Result<String> {
/// # panic!("")
/// }
/// ```
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

    /// Returns the current scope.
    ///
    /// # Example
    /// ```
    /// # use google_cloud_gax_internal::observability::RequestRecorder;
    /// use google_cloud_gax::options::RequestOptions;
    /// async fn sample(options: &RequestOptions, request: reqwest::RequestBuilder) -> anyhow::Result<()> {
    ///     let response = request.send().await?;
    ///     if let Some(current) = RequestRecorder::current() {
    ///         current.on_http_response(&response);
    ///     }
    ///     // ... do something with `response` ...
    ///     Ok(())
    /// }
    /// ```
    pub fn current() -> Option<Self> {
        RECORDER.try_get().ok()
    }

    /// Returns the data captured for the T3 layer.
    ///
    /// # Example
    /// ```
    /// # use google_cloud_gax_internal::observability::RequestRecorder;
    /// async fn emit_l3_log<T>(result: &google_cloud_gax::Result<T>) {
    ///     let Err(e) = result else { return; };
    ///     let Some(recorder) = RequestRecorder::current() else { return; };
    ///     let snapshot = recorder.t3_snapshot();
    ///     tracing::error!(
    ///         { URL_DOMAIN } = snapshot.info.default_host,
    ///         // use more things from snapshot here.
    ///     );
    /// }
    /// ```
    pub fn t3_snapshot(&self) -> T3Snapshot {
        let guard = self.inner.lock().expect("never poisoned");
        guard.clone()
    }

    /// Call before issuing a HTTP request to capture its data.
    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_request(&self, options: &RequestOptions, request: &reqwest::Request) {
        let mut guard = self.inner.lock().expect("never poisoned");
        let snapshot = T4Snapshot {
            start: Instant::now(),
            server_address: None,
            url_template: options.get_extension::<PathTemplate>().map(|e| e.0),
            rpc_system: Some(RPC_SYSTEM_HTTP),
            rpc_method: None,
            http_method: Some(request.method().clone()),
            http_status_code: None,
            url: Some(request.url().to_string()),
        };
        guard.t4_snapshot = Some(snapshot);
    }

    /// Call when receiving a HTTP response to capture its data.
    ///
    /// For theese purpopses, responses that return an error status code are considered successful,
    /// we just need them to capture their data for the spans and metrics.
    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_response(&self, response: &reqwest::Response) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.attempt_count += 1;
        if let Some(s) = guard.t4_snapshot.as_mut() {
            s.server_address = response.remote_addr();
            s.http_status_code = Some(response.status().as_u16());
        }
    }

    /// Call when it was not possible to send an HTTP request.
    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_error(&self, _err: &Error) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.attempt_count += 1;
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

    /// Returns the default host (e.g. `storage.googleapis.com`).
    ///
    /// Use with the "url.domain" attribute.
    pub fn default_host(&self) -> &'static str {
        self.info.default_host
    }

    /// Returns the RPC system (HTTP or gRPC) used in the last low-level request.
    ///
    /// Use with the "rpc.system.name" attribute.
    pub fn rpc_system(&self) -> Option<&'static str> {
        self.t4_snapshot.as_ref().and_then(|s| s.rpc_system)
    }

    /// Returns the URL template (e.g. "/v1/storage/b/{bucket}") used in the last low-level request.
    ///
    /// Use with the "url.template" attribute.
    pub fn url_template(&self) -> Option<&'static str> {
        self.t4_snapshot.as_ref().and_then(|s| s.url_template)
    }

    /// Returns the RPC method (e.g. "cloud.google.secretmanager.v1.SecretManager/GetSecret") used in the request.
    ///
    /// Use with the "rpc.method" attribute.
    pub fn rpc_method(&self) -> Option<&'static str> {
        self.t4_snapshot.as_ref().and_then(|s| s.url_template)
    }

    /// Returns the HTTP status code (e.g. 404) returned in the last request.
    ///
    /// Note that this may not be populated for gRPC requests.
    ///
    /// Use with the "rpc.method" attribute.
    pub fn http_status_code(&self) -> Option<u16> {
        self.t4_snapshot.as_ref().and_then(|s| s.http_status_code)
    }

    /// Returns the full URL used in the last request.
    ///
    /// Note that this may not be populated for gRPC requests.
    ///
    /// Use with the "rpc.method" attribute.
    pub fn url(&self) -> Option<&str> {
        self.t4_snapshot.as_ref().and_then(|s| s.url.as_deref())
    }

    // TODO(#4795) - remove once it is no longer used.
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
    pub rpc_system: Option<&'static str>,
    pub rpc_method: Option<&'static str>,
    pub url_template: Option<&'static str>,
    pub http_method: Option<Method>,
    pub http_status_code: Option<u16>,
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::super::tests::TEST_INFO;
    use super::*;

    #[tokio::test]
    async fn scope() {
        let metric = DurationMetric::new(&TEST_INFO);
        let span = tracing::info_span!("test-only");
        let recorder = RequestRecorder::new(TEST_INFO.clone());

        let scoped = recorder.clone();
        let got = scoped
            .t3_scope(metric, span, async {
                let current =
                    RequestRecorder::current().expect("current recorder should be available");
                let snap = current.t3_snapshot();
                assert_eq!(snap.attempt_count, 0, "{snap:?}");
                assert_eq!(snap.default_host(), TEST_INFO.default_host, "{snap:?}");
                current.on_http_error(&Error::deser("cannot deserialize"));
                Ok(123)
            })
            .await;

        assert!(matches!(got, Ok(ref v) if v == &123), "{got:?}");
        let snap = recorder.t3_snapshot();
        assert_eq!(snap.attempt_count, 1, "{snap:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn on_http_request() -> anyhow::Result<()> {
        let recorder = RequestRecorder::new(TEST_INFO.clone());
        let options = RequestOptions::default().insert_extension(PathTemplate("/v7/{funny}"));
        let client = reqwest::Client::new();
        let request = client.get("http://127.0.0.1:1/v7/will-not-work").build()?;

        recorder.on_http_request(&options, &request);
        let snap = recorder.t3_snapshot();
        assert_eq!(snap.start, Instant::now(), "{snap:?}");
        assert_eq!(snap.url_template(), Some("/v7/{funny}"), "{snap:?}");
        assert_eq!(snap.rpc_system(), Some("http"), "{snap:?}");
        assert!(snap.rpc_method().is_none(), "{snap:?}");
        assert!(snap.http_status_code().is_none(), "{snap:?}");
        assert_eq!(
            snap.url(),
            Some("http://127.0.0.1:1/v7/will-not-work"),
            "{snap:?}"
        );
        Ok(())
    }
}
