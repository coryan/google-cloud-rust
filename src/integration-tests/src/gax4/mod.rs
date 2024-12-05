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
    let response = client.execute(builder, body).await?;
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
        body: reqwest::Body,
    ) -> impl std::future::Future<Output = Result<http::response::Response<reqwest::Body>>> + Send;
}

pub struct ReqwestClient {
    inner: reqwest::Client,
    endpoint: String,
}

impl std::fmt::Debug for ReqwestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Client[{}]", self.endpoint)
    }
}

impl ReqwestClient {
    pub fn default() -> Self {
        Self {
            inner: reqwest::Client::new(),
            endpoint: "https://foo.googleapis.com".to_string(),
        }
    }
}

impl HttpClient for ReqwestClient {
    async fn execute(
        &self,
        builder: http::request::Builder,
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
            .service(self.inner.clone());

        let response = client
            .call(builder.body(body).map_err(Error::io)?)
            .await
            .map_err(Error::io)?;
        Ok(response)
    }
}
