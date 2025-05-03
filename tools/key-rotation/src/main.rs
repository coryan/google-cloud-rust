// Copyright 2025 Google LLC
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

use axum::{
    Router,
    response::{Html, Json},
    routing::get,
};
use google_cloud_gax::{self as gax, retry_policy::RetryPolicyExt};
use google_cloud_iam_admin_v1 as iam;
use google_cloud_wkt as wkt;
use http::StatusCode;
use serde_json::json;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler))
        .route("/rotate", get(rotate_handler));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!(
        "{}",
        json!({"severity": "info", "message": format!("listening on {:?}", listener.local_addr()) })
    );
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<String> {
    let program_name = env!("CARGO_PKG_NAME");
    let program_version = env!("CARGO_PKG_VERSION");
    let service_account = std::env::var("SERVICE_ACCOUNT");
    let secret_id = std::env::var("SECRET_ID");
    Html(format!(
        r###"<!DOCTYPE HTML>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Key Rotation Status</title>
    </head>
    <body>
    <ul>
        <li>Program name: {program_name}</li>
        <li>Program version: {program_version}</li>
        <li>Service Account: {service_account:?}</li>
        <li>Secret ID: {secret_id:?}</li>
    </ul>
    </body>
"###
    ))
}

async fn rotate_handler() -> (StatusCode, Json<serde_json::Value>) {
    let result = rotate().await;
    match result {
        Ok(_) => (StatusCode::OK, Json(json!({}))),
        Err(e) => {
            let error = json!({
                "severity": "ERROR",
                "message": format!("{e}"),
            });
            eprintln!("{}", error.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

async fn rotate() -> anyhow::Result<()> {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")?;
    let service_account = std::env::var("SERVICE_ACCOUNT")?;

    let client = iam::client::Iam::builder()
        .with_retry_policy(
            gax::retry_policy::AlwaysRetry
                .with_attempt_limit(5)
                .with_time_limit(std::time::Duration::from_secs(15)),
        )
        .build()
        .await?;

    // No pagination, the number of keys is limited.
    let list = client
        .list_service_account_keys(format!(
            "projects/{project_id}/serviceAccounts/{service_account}"
        ))
        .send()
        .await?;

    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let now = wkt::Timestamp::new(d.as_secs() as i64, 0)?;
    let refresh_deadline = wkt::Timestamp::new(d.as_secs() as i64 + 7 * 24 * 3600, 0)?;
    let expired: Vec<_> = list
        .keys
        .iter()
        .filter_map(|k| match &k.valid_before_time {
            None => None,
            Some(t) if t >= &now => Some(k.name.clone()),
            Some(_) => None,
        })
        .collect();
    for name in expired {
        client.delete_service_account_key(name).send().await?;
    }

    let expires_last =
        list.keys
            .into_iter()
            .reduce(|a, b| match (&a.valid_before_time, &b.valid_before_time) {
                (None, _) => a,
                (Some(_), None) => b,
                (Some(ta), Some(tb)) if ta >= tb => a,
                (Some(_), Some(_)) => b,
            });
    let up_to_date = match &expires_last {
        None => false,
        Some(key) => key
            .valid_before_time
            .as_ref()
            .and_then(|t| Some(t >= &refresh_deadline))
            .unwrap_or(true),
    };
    if !up_to_date {
        return Ok(());
    }

    Ok(())
}
