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

// #[cfg(all(test, feature = "run-integration-tests"))]
mod prediction {
    use google_cloud_aiplatform_v1::client::PredictionService;
    use google_cloud_aiplatform_v1::model::{Content, Part};
    use google_cloud_gax::options::RequestOptionsBuilder;
    use google_cloud_gax::retry_policy::{AlwaysRetry, RetryPolicyExt};
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn gemini_generate_content() -> anyhow::Result<()> {
        const REGION: &str = "us-central1";
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")?;
        let client = PredictionService::builder()
            .with_endpoint(format!("https://{REGION}-aiplatform.googleapis.com"))
            .with_retry_policy(AlwaysRetry.with_attempt_limit(3))
            .build()
            .await?;

        let model = format!(
            "projects/{project_id}/locations/{REGION}/publishers/google/models/gemini-2.5-flash-lite"
        );
        for _ in 0..1000 {
            let response = client
                .generate_content()
                .with_attempt_timeout(Duration::from_secs(300))
                .set_model(&model)
                .set_contents([Content::new().set_role("user").set_parts([
                    Part::new().set_text("Why do people fall for parlor tricks like LLMs?")
                ])])
                .send()
                .await;
            match response {
                Ok(_r) => {
                    println!("SUCCESS: {:?}", Instant::now());
                }
                Err(e) if format!("{e}").contains("unexpected-eof") => {
                    println!("REPRO: {e:?}");
                    return Err(e.into());
                }
                Err(e) => println!("ERROR (not repro): {e:?}"),
            };
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }
}
