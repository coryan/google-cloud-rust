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

use axum::{extract::State, http::StatusCode, Json};
use gax::http_client::*;
use gax::options::*;
use gcp_sdk_gax as gax;
use serde_json::{json, Value};
use std::collections::VecDeque;
use tokio::task::JoinHandle;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Response = (StatusCode, Value);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_no_error() -> Result<()> {
    let responses: VecDeque<_> = [(
        StatusCode::OK,
        json!({ "name": "projects/test-only/foos/1", "value": "v" }),
    )]
    .into_iter()
    .collect();
    let (endpoint, _server) = start(responses).await?;
    let config = ClientConfig::default().set_credential(auth::Credential::test_credentials());
    let client = ReqwestClient::new(config, &endpoint).await?;

    let builder = client
        .builder(reqwest::Method::GET, "/get".into())
        .query(&[("name", "projects/test-only/foo/1")]);
    let response = client
        .execute_with_options::<serde_json::Value, serde_json::Value>(
            builder,
            None,
            RequestOptions::new(),
        )
        .await?;

    assert_eq!(
        response,
        json!({ "name": "projects/test-only/foos/1", "value": "v"})
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retry_transient_errors() -> Result<()> {
    let responses: VecDeque<_> = [
        (StatusCode::BAD_REQUEST, unavailable()),
        (StatusCode::BAD_REQUEST, unavailable()),
        (StatusCode::OK, success()),
    ]
    .into_iter()
    .collect();
    let (endpoint, _server) = start(responses).await?;
    let config = ClientConfig::default()
    .set_credential(auth::Credential::test_credentials());
    let client = ReqwestClient::new(config, &endpoint).await?;

    let builder = client
        .builder(reqwest::Method::GET, "/get".into())
        .query(&[("name", "projects/test-only/foo/1")]);
    let options = gax::options::RequestOptions::new().set::<gax::options::RetryPolicy>(
        gax::retry::new_provider(
        gax::retry::LimitedErrorCount::new(3)
    ));
    let response = client
        .execute_with_options::<serde_json::Value, serde_json::Value>(
            builder,
            None,
            options,
        )
        .await?;

    assert_eq!(
        response,
        json!({ "name": "projects/test-only/foos/1", "value": "v"})
    );

    Ok(())
}

fn success() -> serde_json::Value {
    json!({ "name": "projects/test-only/foos/1", "value": "v" })
}

fn unavailable() -> serde_json::Value {
    json!({
        "error": {
            "code": 400, "status":
            "UNAVAILABLE",
            "message": "service temporarily unavailable",
            "details": [
                {"@type": "google.rpc.RetryInfo", "retry_delay": "1.5s" }
            ]
        }
    })
}

pub async fn start(responses: VecDeque<Response>) -> Result<(String, JoinHandle<()>)> {
    let app = axum::Router::new()
        .route("/get", axum::routing::get(handler))
        .with_state(responses);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async {
        axum::serve(listener, app).await.unwrap();
    });

    Ok((format!("http://{}:{}", addr.ip(), addr.port()), server))
}

async fn handler(
    State(responses): &mut State<VecDeque<Response>>,
) -> (StatusCode, Json<serde_json::Value>) {
    match responses.pop_front() {
        Some(r) => (r.0, Json::from(r.1)),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json::from(serde_json::Value::String("out of responses".into())),
        ),
    }
}
