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

pub async fn execute<I: serde::ser::Serialize, O: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    builder: http::request::Builder,
    path: String,
    body: Option<I>,
) -> Result<O> {
    let body = body
        .map(|p| serde_json::to_vec(&p))
        .transpose()
        .map_err(Error::serde)?;
    let body = match body {
        None => reqwest::Body::default(),
        Some(_v) => reqwest::Body::default(),
    };
    let response = client.execute(builder, path, body).await?;
    // Handle HTTP errors here ..
    // Handle the case when body().as_bytes() is None.
    let response =
        serde_json::from_slice(response.body().as_bytes().unwrap()).map_err(Error::serde)?;
    Ok(response)
}

pub trait HttpClient: std::fmt::Debug + Send + Sync {
    fn execute(
        &self,
        builder: http::request::Builder,
        path: String,
        body: reqwest::Body,
    ) -> impl std::future::Future<Output = Result<http::response::Response<reqwest::Body>>> + Send;
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

impl<E> tower::retry::Policy<reqwest::Request, reqwest::Response, E>
    for Attempts
{
    type Future = std::future::Ready<()>;

    fn retry(
        &mut self,
        _req: &mut reqwest::Request,
        result: &mut std::result::Result<reqwest::Response, E>,
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

    fn clone_request(&mut self, _req: &reqwest::Request) -> Option<reqwest::Request> {
        None
    }
}

impl HttpClient for ReqwestClient {
    async fn execute(
        &self,
        builder: http::request::Builder,
        path: String,
        body: reqwest::Body,
    ) -> Result<http::response::Response<reqwest::Body>> {
        use tower::Service;
        use tower_http::ServiceBuilderExt;

        let mut client = tower::ServiceBuilder::new()
            .override_request_header(
                http::header::USER_AGENT,
                http::HeaderValue::from_static("test-with-tower"),
            )
            .timeout(std::time::Duration::new(60, 0))
            .layer(tower_reqwest::HttpClientLayer)
            .retry(Attempts(7))
            .service(self.inner.clone());

        let request = builder
            .uri(format!("{}/{}", &self.endpoint, &path))
            .body(body)
            .map_err(Error::io)?;
        let response = client.call(request).await.map_err(Error::io)?;
        Ok(response)
    }
}
