// Copyright 2026 Google LLC
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

use google_cloud_aiplatform_v1::client::PredictionService;
use google_cloud_aiplatform_v1::model::{Content, GenerationConfig, Part};
use google_cloud_gax::error::rpc::Code;
use google_cloud_gax::options::RequestOptionsBuilder;
use google_cloud_gax::retry_policy::{AlwaysRetry, RetryPolicyExt};
use std::sync::atomic::{AtomicU64, Ordering::AcqRel, Ordering::Acquire};
use std::time::Duration;

const REGION: &str = "us-central1";
const USER_COUNT: usize = 256;

static SUCCESS_COUNT: AtomicU64 = AtomicU64::new(0);
static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);
static RESOURCE_EXHAUSTED_COUNT: AtomicU64 = AtomicU64::new(0);
static REPRO_COUNT: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")?;
    let client = PredictionService::builder()
        .with_endpoint(format!("https://{REGION}-aiplatform.googleapis.com"))
        .with_retry_policy(AlwaysRetry.with_time_limit(Duration::from_secs(300)))
        .build()
        .await?;

    // Verify we have at least 4 worker threads. It is boring otherwise.
    let runtime = tokio::runtime::Handle::try_current()?;
    assert!(
        runtime.metrics().num_workers() > 4,
        "metrics={:?}",
        runtime.metrics().num_workers()
    );

    let tasks = (0..USER_COUNT)
        .map(|i| simulate_user(client.clone(), project_id.clone(), i))
        .collect::<Vec<_>>();
    let tasks = tokio::spawn(futures::future::join_all(tasks)).await?;
    for (i, t) in tasks.into_iter().enumerate() {
        if let Err(e) = t {
            println!("task {i:04} failed with: {e:?}");
        }
    }

    Ok(())
}

async fn simulate_user(
    client: PredictionService,
    project_id: String,
    id: usize,
) -> anyhow::Result<()> {
    let model = format!(
        "projects/{project_id}/locations/{REGION}/publishers/google/models/gemini-2.5-flash-lite"
    );
    for iteration in 0..1000 {
        if id == 0 && iteration > 0 && iteration % 10 == 0 {
            [
                ("SUCCESS", &SUCCESS_COUNT),
                ("REPRO", &REPRO_COUNT),
                ("RESOURCE_EXHAUSTED", &RESOURCE_EXHAUSTED_COUNT),
                ("ERROR", &ERROR_COUNT),
            ]
            .iter()
            .for_each(|(name, count)| {
                println!("{name} = {}", count.load(Acquire));
            });
        }
        let response = client
            .generate_content()
            .with_attempt_timeout(Duration::from_secs(300))
            .set_model(&model)
            .set_contents([Content::new()
                .set_role("user")
                .set_parts([Part::new().set_text(
                "What is the most efficient way to handle high-concurrency JSON parsing in Rust?",
            )])])
            .set_system_instruction(
                Content::new()
                    .set_role("system")
                    .set_parts([Part::new().set_text("You are a systems performance expert.")]),
            )
            .set_generation_config(
                GenerationConfig::new()
                    .set_temperature(0.5)
                    .set_max_output_tokens(8192)
                    .set_response_mime_type("application/json"),
            )
            .send()
            .await;
        match response {
            Ok(r) => {
                let count = SUCCESS_COUNT.fetch_add(1, AcqRel);
                if count == 1 {
                    println!("[{id:04}] SUCCESS : {r:?}");
                }
            }
            Err(e) if format!("{e}").contains("unexpected-eof") => {
                REPRO_COUNT.fetch_add(1, AcqRel);
                println!("[{id:04}] REPRO  : {e:?}");
                return Err(e.into());
            }
            Err(e)
                if e.status()
                    .is_some_and(|s| s.code == Code::ResourceExhausted) =>
            {
                RESOURCE_EXHAUSTED_COUNT.fetch_add(1, AcqRel);
            }
            Err(e) => {
                ERROR_COUNT.fetch_add(1, AcqRel);
                println!("[{id:04}] ERROR  : {e:?}");
            }
        };
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
