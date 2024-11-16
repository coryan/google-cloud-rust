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

use std::error::Error;
use std::result::Result;
use client_as_trait::*;

pub async fn secretmanager() -> Result<(), Box<dyn Error>> {
    let client = DefaultSecretManagerService::default().await?;
    let response = client
        .list_secrets(sm::model::ListSecretsRequest::default().set_parent("projects/coryan-test"))
        .await?;
    println!("RESPONSE = {response:?}");

    Ok(())
}

#[cfg(all(test, feature = "run-integration-tests"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn run_secretmanager() -> Result<(), Box<dyn Error>> {
    secretmanager().await?;
    Ok(())
}
