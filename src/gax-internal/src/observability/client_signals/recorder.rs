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

use crate::observability::RequestStart;
use crate::options::InstrumentationClientInfo;
#[cfg(feature = "_internal-http-client")]
use google_cloud_gax::error::Error;
use google_cloud_gax::options::RequestOptions;
use google_cloud_gax::options::internal::{PathTemplate, RequestOptionsExt};
use reqwest::Method;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

tokio::task_local! {
    static RECORDER: RequestRecorder;
}

/// Collects key information about a request to update the telemetry information.
///
/// The name should evoke a "flight recorder" the devices to records interesting events about airplane operations.
#[derive(Clone, Debug)]
pub struct RequestRecorder {
    inner: Arc<Mutex<RecorderInner>>,
}

#[derive(Clone, Debug)]
struct RecorderInner {
    t3_start: Instant,
    info: InstrumentationClientInfo,
    attempt_count: u32,
    t4_start: Option<Instant>,
    server_address: Option<SocketAddr>,
    http_method: Option<Method>,
    url: Option<String>,
    url_template: Option<&'static str>,
    rpc_method: Option<&'static str>,
}

impl RequestRecorder {
    pub fn new(info: InstrumentationClientInfo) -> Self {
        let inner = RecorderInner::new(info);
        let inner = Arc::new(Mutex::new(inner));
        Self { inner }
    }

    pub fn record<F>(&self, future: F) -> tokio::task::futures::TaskLocalFuture<Self, F>
    where
        F: std::future::Future,
    {
        RECORDER.scope(self.clone(), future)
    }

    pub fn current() -> Option<Self> {
        RECORDER.try_get().ok()
    }

    pub fn t3_start(&self) -> RequestStart {
        let guard = self.inner.lock().expect("never poisoned");
        RequestStart::from_parts(
            guard.t3_start,
            guard.info.clone(),
            guard.url_template.unwrap_or_default(),
            guard.rpc_method.unwrap_or_default(),
        )
    }

    pub fn extend(&self, options: RequestOptions) -> RequestOptions {
        use google_cloud_gax::options::internal::RequestOptionsExt;
        options.insert_extension(self.clone())
    }

    pub fn server_address(&self) -> Option<SocketAddr> {
        self.inner
            .lock()
            .expect("never poisoned")
            .server_address
            .clone()
    }

    pub fn method(&self) -> Option<Method> {
        self.inner
            .lock()
            .expect("never poisoned")
            .http_method
            .clone()
    }

    pub fn url(&self) -> Option<String> {
        self.inner.lock().expect("never poisoned").url.clone()
    }

    pub fn attempt_count(&self) -> u32 {
        self.inner.lock().expect("never poisoned").attempt_count
    }

    pub fn last_server_address(&self, address: Option<SocketAddr>) {
        if let Some(a) = address {
            let mut guard = self.inner.lock().expect("never poisoned");
            guard.server_address = Some(a);
        }
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_request(&self, options: &RequestOptions, request: &reqwest::Request) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.t4_start = Some(Instant::now());
        guard.url_template = options.get_extension::<PathTemplate>().map(|e| e.0);
        guard.http_method = Some(request.method().clone());
        guard.url = Some(request.url().to_string());
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_response(&self, response: &reqwest::Response) {
        let mut guard = self.inner.lock().expect("never poisoned");
        guard.server_address = response.remote_addr();
        guard.attempt_count += 1;
    }

    #[cfg(feature = "_internal-http-client")]
    pub fn on_http_error(&self, _err: &Error) {
        let mut guard = self.inner.lock().expect("never poisoned");
        println!("#### -> on_http_error: {_err:?}");
        guard.server_address = None;
        guard.attempt_count += 1;
    }
}

impl RecorderInner {
    fn new(info: InstrumentationClientInfo) -> Self {
        let t3_start = Instant::now();
        Self {
            t3_start,
            info,
            attempt_count: 0,
            t4_start: None,
            server_address: None,
            http_method: None,
            url: None,
            url_template: None,
            rpc_method: None,
        }
    }
}
