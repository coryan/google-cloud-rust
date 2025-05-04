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

use axum::{Router, response::Json, routing::get};
use google_cloud_gax::{self as gax, retry_policy::RetryPolicyExt};
use google_cloud_iam_admin_v1 as iam;
use google_cloud_secretmanager_v1 as sm;
use google_cloud_wkt as wkt;
use http::StatusCode;
use serde_json::{Value, json};

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

async fn handler() -> Json<Value> {
    let response = json!({
        "programName": env!("CARGO_PKG_NAME"),
        "programVersion": env!("CARGO_PKG_VERSION"),
        "serviceAccount": format!("{:?}", std::env::var("SERVICE_ACCOUNT")),
        "secretId": format!("{:?}", std::env::var("SECRET_ID")),
    });
    Json(response)
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
            eprintln!("{}", error);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

async fn rotate() -> anyhow::Result<Value> {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")?;
    let service_account_id = std::env::var("SERVICE_ACCOUNT")?;
    let service_account_email =
        format!("{service_account_id}@{project_id}.iam.gserviceaccount.com");

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
            "projects/{project_id}/serviceAccounts/{service_account_email}"
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
    for name in expired.iter() {
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
            .map(|t| t >= &refresh_deadline)
            .unwrap_or(true),
    };
    if !up_to_date {
        return Ok(json!({"expiredKeys":  expired}));
    }

    let key = client
        .create_service_account_key(format!(
            "projects/{project_id}/serviceAccounts/{service_account_email}"
        ))
        .send()
        .await?;

    let secret_version = update_secret(&project_id, key.private_key_data).await?;

    Ok(json!({"expiredKeys": expired, "newKey": key.name, "newSecretVersion": secret_version }))
}

async fn update_secret(project_id: &str, data: bytes::Bytes) -> anyhow::Result<String> {
    let secret_id = std::env::var("SECRET_ID")?;

    let client = sm::client::SecretManagerService::builder()
        .with_retry_policy(
            gax::retry_policy::AlwaysRetry
                .with_attempt_limit(5)
                .with_time_limit(std::time::Duration::from_secs(15)),
        )
        .build()
        .await?;

    let version = create_version(&client, project_id, &secret_id, data.clone()).await;
    match version {
        Ok(name) => Ok(name),
        Err(e) => {
            match e.as_inner::<gax::error::ServiceError>() {
                // It is more efficient to create the parent secret only if the
                // request fails with NotFound.
                Some(svc) if is_not_found(svc.status()) => {
                    create_secret(&client, project_id, &secret_id).await?;
                    let name = create_version(&client, project_id, &secret_id, data).await?;
                    Ok(name)        
                },
                _ => Err(e.into()),
            }
            
        },
    }
}

async fn create_version(
    client: &sm::client::SecretManagerService,
    project_id: &str,
    secret_id: &str,
    data: bytes::Bytes,
) -> gax::Result<String> {
    let checksum = crc32c::crc32c(&data);
    let response = client
        .add_secret_version(format!("projects/{project_id}/secrets/{secret_id}"))
        .set_payload(
            sm::model::SecretPayload::new()
                .set_data(data.clone())
                .set_data_crc32c(checksum as i64),
        )
        .send()
        .await?;
    Ok(response.name)
}

async fn create_secret(
    client: &sm::client::SecretManagerService,
    project_id: &str,
    secret_id: &str,
) -> gax::Result<()> {
    let _ = client
        .create_secret(format!("projects/{project_id}"))
        .set_secret_id(secret_id)
        .set_secret(sm::model::Secret::new().set_replication(
            sm::model::Replication::new().set_automatic(sm::model::replication::Automatic::new()),
        ))
        .send()
        .await?;
    Ok(())
}

fn is_not_found(status: &gax::error::rpc::Status) -> bool {
    return status.code == 404 || status.code == gax::error::rpc::Code::NotFound as i32
}
