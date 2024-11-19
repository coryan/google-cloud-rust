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

use client_as_trait::*;
use rand::{distributions::Alphanumeric, Rng};
use std::error::Error;
use std::result::Result;

const PROJECT_ID: &str = "coryan-test";
const SECRET_ID_LENGTH: usize = 64;

pub async fn secretmanager() -> Result<(), Box<dyn Error>> {
    let secret_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SECRET_ID_LENGTH)
        .map(char::from)
        .collect();

    let client = DefaultSecretManagerService::default().await?;

    cleanup_stale_secrets(&client, &secret_id).await?;

    let response = client
        .create_secret(
            sm::model::CreateSecretRequest::default()
                .set_parent(format!("projects/{PROJECT_ID}"))
                .set_secret_id(&secret_id)
                .set_secret(
                    sm::model::Secret::default()
                        .set_replication(sm::model::Replication::default().set_replication(
                            sm::model::replication::Replication::Automatic(
                                sm::model::replication::Automatic::default(),
                            ),
                        ))
                        .set_labels(
                            [("integration-test", "true")]
                                .map(|(k, v)| (k.to_string(), v.to_string())),
                        ),
                ),
        )
        .await?;
    println!("CREATE = {response:?}");

    assert_eq!(
        response.name,
        format!("projects/{PROJECT_ID}/secrets/{secret_id}")
    );

    let response = client
        .delete_secret(sm::model::DeleteSecretRequest::default().set_name(response.name))
        .await?;
    println!("DELETE = {response:?}");

    Ok(())
}

pub async fn cleanup_stale_secrets(
    client: &DefaultSecretManagerService,
    secret_id: &str,
) -> Result<(), Box<dyn Error>> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stale_deadline = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let stale_deadline = stale_deadline - std::time::Duration::from_secs(48 * 60 * 60);
    let stale_deadline = stale_deadline.as_secs() as i64;

    let mut stale_secrets = Vec::new();
    let mut list_request =
        sm::model::ListSecretsRequest::default().set_parent(format!("projects/{PROJECT_ID}"));
    loop {
        let response = client.list_secrets(list_request.clone()).await?;
        for secret in response.secrets {
            if secret
                .name
                .ends_with(format!("/secrets/{secret_id}").as_str())
            {
                return Err("randomly generated secret id already exists {secret_id}".into());
            }

            if let Some("true") = secret.labels.get("integration-test").map(String::as_str) {
                if let Some(timestamp) = secret.create_time {
                    if timestamp.seconds < stale_deadline {
                        stale_secrets.push(secret.name);
                    }
                }
            }
        }
        if response.next_page_token.is_empty() {
            break;
        }
        list_request.page_token = response.next_page_token;
    }

    let pending = stale_secrets
        .iter()
        .map(|v| client.delete_secret(sm::model::DeleteSecretRequest::default().set_name(v)))
        .collect::<Vec<_>>();

    // Print the errors, but otherwise ignore them.
    futures::future::join_all(pending)
        .await
        .into_iter()
        .zip(stale_secrets)
        .for_each(|(r, name)| println!("{name} = {r:?}"));

    Ok(())
}

#[cfg(all(test, feature = "run-integration-tests"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn run_secretmanager() -> Result<(), Box<dyn Error>> {
    secretmanager().await?;
    Ok(())
}
