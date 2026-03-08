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

use std::time::Duration;

use super::args::Args;
use google_cloud_aiplatform_v1::client::PredictionService;
use google_cloud_auth::credentials::Credentials;
use google_cloud_gax::retry_policy::{Aip194Strict, RetryPolicyExt};

#[derive(Clone, Debug)]
pub struct AppState {
    args: Args,
    prediction_service: PredictionService,
}

impl AppState {
    pub async fn new(args: Args, credentials: Credentials) -> anyhow::Result<Self> {
        let prediction_service = PredictionService::builder()
            .with_credentials(credentials)
            .with_retry_policy(
                Aip194Strict
                    .continue_on_too_many_requests()
                    .with_time_limit(Duration::from_secs(15)),
            )
            .with_tracing()
            .build()
            .await?;
        Ok(Self {
            args,
            prediction_service,
        })
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn prediction_service(&self) -> &PredictionService {
        &self.prediction_service
    }
}
