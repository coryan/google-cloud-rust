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

use gax::http_client::*;
use gax::options::*;
use gcp_sdk_gax as gax;
use serde_json::json;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod echo_server;

#[tokio::test(start_paused = true)]
async fn test_no_timeout() -> Result<()> {
    let (endpoint, server) = echo_server::start().await?;
    let config = ClientConfig::default().set_credential(auth::Credential::test_credentials());
    let client = ReqwestClient::new(config, &endpoint).await?;

    let delay = Duration::from_millis(200);
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let builder = client
        .builder(reqwest::Method::GET, "/echo".into())
        .query(&[("delay", format!("{}", delay.as_millis()))]);
    let response = client.execute_with_options::<serde_json::Value, serde_json::Value>(
        builder,
        Some(json!({})),
        RequestOptions::new(),
    );

    tokio::pin!(server);
    tokio::pin!(response);
    loop {
        tokio::select! {
            _ = &mut server => { },
            response = &mut response => {
                let response = response?;
                assert_eq!(
                    echo_server::get_query_value(&response, "delay"),
                    Some("200".to_string())
                );
                break;
            },
            _ = interval.tick() => { },
        }
    }

    Ok(())
}

#[tokio::test(start_paused = true)]
async fn test_timeout_does_not_expire() -> Result<()> {
    let (endpoint, server) = echo_server::start().await?;
    let config = ClientConfig::default().set_credential(auth::Credential::test_credentials());
    let client = ReqwestClient::new(config, &endpoint).await?;

    let delay = Duration::from_millis(200);
    let timeout = Duration::from_millis(2000);
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let builder = client
        .builder(reqwest::Method::GET, "/echo".into())
        .query(&[("delay", format!("{}", delay.as_millis()))]);
    let response = client.execute_with_options::<serde_json::Value, serde_json::Value>(
        builder,
        Some(json!({})),
        RequestOptions::new().set::<RequestTimeout>(timeout),
    );

    tokio::pin!(server);
    tokio::pin!(response);
    loop {
        tokio::select! {
            _ = &mut server => {  },
            response = &mut response => {
                let response = response?;
                assert_eq!(
                    echo_server::get_query_value(&response, "delay"),
                    Some("200".to_string())
                );
                break;
            },
            _ = interval.tick() => { },
        }
    }

    Ok(())
}

#[tokio::test(start_paused = true)]
async fn test_timeout_expires() -> Result<()> {
    let (endpoint, server) = echo_server::start().await?;
    let config = ClientConfig::default().set_credential(auth::Credential::test_credentials());
    let client = ReqwestClient::new(config, &endpoint).await?;

    let delay = Duration::from_millis(200);
    let timeout = Duration::from_millis(150);
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let builder = client
        .builder(reqwest::Method::GET, "/echo".into())
        .query(&[("delay", format!("{}", delay.as_millis()))]);
    let response = client.execute_with_options::<serde_json::Value, serde_json::Value>(
        builder,
        Some(json!({})),
        RequestOptions::new().set::<RequestTimeout>(timeout),
    );

    tokio::pin!(server);
    tokio::pin!(response);
    loop {
        tokio::select! {
            _ = &mut server => {  },
            response = &mut response => {
                use gax::error::ErrorKind;
                assert!(
                    response.is_err(),
                    "expected an error when timeout={}, got={:?}",
                    timeout.as_millis(),
                    response
                );
                let err = response.err().unwrap();
                assert_eq!(err.kind(), ErrorKind::Io);
                break;
            },
            _ = interval.tick() => { },
        }
    }

    Ok(())
}
