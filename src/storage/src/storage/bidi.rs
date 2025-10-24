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

use crate::storage::client::ClientBuilder;

use super::request_options::RequestOptions;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bidi<S = BidiTransport> {
    stub: std::sync::Arc<S>,
    options: RequestOptions,
}

impl Bidi {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub(crate) async fn new(
        builder: super::client::ClientBuilder,
    ) -> gax::client_builder::Result<Self> {
        let (client_config, options) = builder.into_client_config();
        let client = gaxi::grpc::Client::new(client_config, super::DEFAULT_HOST).await?;
        let stub = Arc::new(BidiTransport::new(client));
        Ok(Self { stub, options })
    }
}

trait BidiStub: std::fmt::Debug + Send + Sync {}

#[derive(Debug)]
pub struct BidiTransport {
    client: gaxi::grpc::Client,
}

impl BidiTransport {
    pub fn new(client: gaxi::grpc::Client) -> Self {
        Self { client }
    }
}

impl BidiStub for BidiTransport {}

impl super::client::ClientBuilder {
    pub async fn build_bidi(self) -> gax::client_builder::Result<Bidi> {
        Bidi::new(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth::credentials::anonymous::Builder as Anonymous;

    #[tokio::test]
    async fn create_new_client() -> anyhow::Result<()> {
        let _client = Bidi::builder()
            .with_credentials(Anonymous::new().build())
            .build_bidi()
            .await?;
        Ok(())
    }
}
