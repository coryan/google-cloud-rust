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

mod builder;
mod object_descriptor;
mod pending_range;
mod range_reader;
pub mod stub;
mod transport;

pub use crate::request_options::RequestOptions;
use crate::storage::client::ClientBuilder;
pub use builder::OpenObject;
#[allow(unused_imports)]
pub use object_descriptor::ObjectDescriptor;
pub use range_reader::RangeReader;
use transport::{ObjectDescriptorTransport, Reconnect};

#[derive(Clone, Debug)]
pub struct Bidi {
    client: gaxi::grpc::Client,
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
        Ok(Self { client, options })
    }

    pub fn open_object<B, O>(&self, bucket: B, object: O) -> OpenObject
    where
        B: Into<String>,
        O: Into<String>,
    {
        OpenObject::new(
            bucket.into(),
            object.into(),
            self.client.clone(),
            self.options.clone(),
        )
    }
}

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
