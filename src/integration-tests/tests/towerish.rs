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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn tower_main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use http_body_util::BodyExt;
    use tower::{Service, ServiceBuilder};
    use tower_http::ServiceBuilderExt;
    use tower_reqwest::HttpClientLayer;

    let mut client = ServiceBuilder::new()
        .override_request_header(
            http::header::USER_AGENT,
            http::HeaderValue::from_static("test-with-tower"),
        )
        .timeout(std::time::Duration::new(60, 0))
        .layer(HttpClientLayer)
        .service(reqwest::Client::new());
    // Execute request by using this service.
    let response = client
        .call(
            http::request::Builder::new()
                .method(http::Method::GET)
                .uri("http://ip.jsontest.com")
                .body(reqwest::Body::default())?,
        )
        .await?;

    let bytes = response.into_body().collect().await?.to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    println!("{value:#?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reqwest_main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let request = client
        .request(reqwest::Method::GET, "http://ip.jsontest.com")
        .build()?;

    let response = client.execute(request).await?;
    let bytes = response.bytes().await?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    println!("{value:#?}");

    Ok(())
}
