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

use google_cloud_gax::{self as gax, retry_policy::RetryPolicyExt};
use google_cloud_iam_admin_v1 as iam;
use google_cloud_secretmanager_v1 as sm;
use google_cloud_wkt as wkt;
use serde_json::json;

#[tokio::main]
async fn main() {
    let result = rotate().await;
    match result {
        Ok(_) => {}
        Err(e) => {
            let error = json!({
                "severity": "ERROR",
                "message": format!("{e}"),
            });
            eprintln!("{}", error.to_string())
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
    let deadline = wkt::Timestamp::new(d.as_secs() as i64, 0)?;
    let expired: Vec<_> = list
        .keys
        .iter()
        .filter_map(|k| match &k.valid_before_time {
            None => None,
            Some(t) if t >= &deadline => Some(k.name.clone()),
            Some(_) => None,
        })
        .collect();
    for name in expired {
        client.delete_service_account_key(name).send().await?;
    }

    let most_recent =
        list.keys
            .into_iter()
            .reduce(|a, b| match (&a.valid_before_time, &b.valid_before_time) {
                (None, _) => a,
                (Some(_), None) => b,
                (Some(ta), Some(tb)) if ta >= tb => a,
                (Some(_), Some(_)) => b,
            });

    Ok(())
}
