// Copyright 2024 Google LLC
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

use gax::error::Error;
type Result<T> = std::result::Result<T, Error>;

type HttpRequest = http::request::Request<reqwest::Body>;
type HttpResponse = http::response::Response<reqwest::Body>;

pub async fn execute<I: serde::ser::Serialize, O: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    builder: http::request::Builder,
    path: String,
    body: Option<I>,
    config: Config,
) -> Result<O> {
    let body = body
        .map(|p| serde_json::to_string(&p))
        .transpose()
        .map_err(Error::serde)?;
    let body = body.map(reqwest::Body::wrap).unwrap_or_else( || reqwest::Body::default());
    let response = client.execute(Request { builder, path, body, config} ).await?;
    // Handle HTTP errors here ..
    // Handle the case when body().as_bytes() is None.
    let response =
        serde_json::from_slice(response.body().as_bytes().unwrap()).map_err(Error::serde)?;
    Ok(response)
}

pub trait HttpClient: std::fmt::Debug + Send + Sync {
    fn execute(
        &self,
        request: Request,
    ) -> impl std::future::Future<Output = Result<http::response::Response<reqwest::Body>>> + Send;
}

#[derive(Debug)]
pub struct Request {
    builder: http::request::Builder,
    path: String,
    body: reqwest::Body,
    config: Config,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub with_retry: bool,
    pub retry_attempts: Option<i32>, // None means "use the default count"
    pub timeout: Option<std::time::Duration>,
    pub user_agent: Option<String>,
}

pub struct ReqwestClient {
    inner: reqwest::Client,
    endpoint: String,
}

impl std::fmt::Debug for ReqwestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("ReqwestClient")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl ReqwestClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            inner: reqwest::Client::new(),
            endpoint: endpoint,
        }
    }
}


#[derive(Clone)]
struct Attempts(usize);

impl<E> tower::retry::Policy<HttpRequest, HttpResponse, E> for Attempts {
    type Future = std::future::Ready<()>;

    fn retry(
        &mut self,
        _req: &mut HttpRequest,
        result: &mut std::result::Result<HttpResponse, E>,
    ) -> Option<Self::Future> {
        match result {
            Ok(_) => {
                // Treat all `Response`s as success,
                // so don't retry...
                None
            }
            Err(_) => {
                // Treat all errors as failures...
                // But we limit the number of attempts...
                if self.0 > 0 {
                    // Try again!
                    self.0 -= 1;
                    Some(std::future::ready(()))
                } else {
                    // Used all our attempts, no retry...
                    None
                }
            }
        }
    }

    fn clone_request(&mut self, req: &HttpRequest) -> Option<HttpRequest> {
        None
    }
}

impl HttpClient for ReqwestClient {
    async fn execute(
        &self,
        request: Request,
    ) -> Result<http::response::Response<reqwest::Body>> {
        use tower::Service;
        use tower_http::ServiceBuilderExt;

        let _timeout = request.config.timeout.map(tower::timeout::TimeoutLayer::new);
        let retry = request.config.with_retry.then( || request.config.retry_attempts.unwrap_or(3) )
        .map(|a| Attempts(a as usize))
        .map(tower::retry::RetryLayer::new);

        let mut client = tower::ServiceBuilder::new()
            .override_request_header(
                http::header::USER_AGENT,
                http::HeaderValue::from_static("test-with-tower"),
            )
            .option_layer(retry)
            // UNCOMMENT THIS BREAKS: .option_layer(_timeout)
            // UNCOMMENT THIS WORKS: .timeout(request.config.timeout.unwrap_or(std::time::Duration::new(10, 0)))
            .layer(tower_reqwest::HttpClientLayer)
            .service(self.inner.clone());

        let req = request.builder
            .uri(format!("{}/{}", &self.endpoint, &request.path))
            .body(request.body)
            .map_err(Error::io)?;
        let response = client.call(req).await.map_err(Error::io)?;
        Ok(response)
    }
}
