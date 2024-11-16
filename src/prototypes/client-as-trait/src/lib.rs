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

use sm::model;
mod client_options;
pub use client_options::ClientOptions;
mod default;
pub use default::DefaultSecretManagerService;
pub mod tokens;

/// Secret Manager Service
///
/// Manages secrets and operations using those secrets. Implements a REST
/// model with the following objects:
///
/// * [Secret](crate::model::Secret)
#[async_trait::async_trait]
pub trait SecretManagerService {
    /// Lists [Secrets][google.cloud.secretmanager.v1.Secret].
    async fn list_secrets(
        &self,
        req: crate::model::ListSecretsRequest,
    ) -> Result<crate::model::ListSecretsResponse, Box<dyn std::error::Error>>;

    /// Creates a new [Secret][google.cloud.secretmanager.v1.Secret] containing no
    /// [SecretVersions][google.cloud.secretmanager.v1.SecretVersion].
    async fn create_secret(
        &self,
        req: crate::model::CreateSecretRequest,
    ) -> Result<crate::model::Secret, Box<dyn std::error::Error>>;

    /// Gets metadata for a given [Secret][google.cloud.secretmanager.v1.Secret].
    async fn get_secret(
        &self,
        req: crate::model::GetSecretRequest,
    ) -> Result<crate::model::Secret, Box<dyn std::error::Error>>;

    /// Deletes a [Secret][google.cloud.secretmanager.v1.Secret].
    async fn delete_secret(
        &self,
        req: crate::model::DeleteSecretRequest,
    ) -> Result<wkt::Empty, Box<dyn std::error::Error>>;
}
