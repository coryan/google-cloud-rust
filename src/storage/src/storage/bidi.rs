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
mod object;

use super::request_options::RequestOptions;
use crate::Result;
use crate::google::storage::v2::{BidiReadObjectRequest, BidiReadObjectResponse};
use crate::storage::client::ClientBuilder;
use builder::OpenObject;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

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
        println!(
            "DEBUG DEBUG client_config={client_config:?} default={}",
            super::DEFAULT_HOST
        );
        let client = gaxi::grpc::Client::new(client_config, super::DEFAULT_HOST).await?;
        println!("DEBUG DEBUG client={client:?}");
        let stub = Arc::new(BidiTransport::new(client));
        Ok(Self { stub, options })
    }
}

impl<S> Bidi<S>
where
    S: BidiStub,
{
    pub fn open_object<B, O>(&self, bucket: B, object: O) -> OpenObject<S>
    where
        B: Into<String>,
        O: Into<String>,
    {
        OpenObject::new(
            bucket.into(),
            object.into(),
            self.stub.clone(),
            self.options.clone(),
        )
    }
}

pub trait BidiStub: std::fmt::Debug + Send + Sync {
    fn streaming_read(
        &self,
        bucket_name: &str,
        rx: Receiver<BidiReadObjectRequest>,
        options: gax::options::RequestOptions,
    ) -> impl Future<Output = Result<tonic::Response<tonic::codec::Streaming<BidiReadObjectResponse>>>>;
}

#[derive(Debug)]
pub struct BidiTransport {
    client: gaxi::grpc::Client,
}

impl BidiTransport {
    pub fn new(client: gaxi::grpc::Client) -> Self {
        Self { client }
    }
}

impl BidiStub for BidiTransport {
    async fn streaming_read(
        &self,
        bucket_name: &str,
        rx: Receiver<BidiReadObjectRequest>,
        options: gax::options::RequestOptions,
    ) -> Result<tonic::Response<tonic::codec::Streaming<BidiReadObjectResponse>>> {
        println!("DEBUG DEBUG - streaming_read(self={self:?} {bucket_name})");
        let options = gax::options::internal::set_default_idempotency(options, true);
        let extensions = {
            let mut e = tonic::Extensions::new();
            e.insert(tonic::GrpcMethod::new(
                "google.storage.v2.Storage",
                "BidiReadObject",
            ));
            e
        };
        let path =
            http::uri::PathAndQuery::from_static("/google.storage.v2.Storage/BidiReadObject");
        let x_goog_request_params = format!("bucket={bucket_name}");
        println!("DEBUG DEBUG - streaming_read({bucket_name}) - {x_goog_request_params}");
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        self.client
            .bidi_stream::<BidiReadObjectRequest, BidiReadObjectResponse>(
                extensions,
                path,
                stream,
                options,
                &super::info::X_GOOG_API_CLIENT_HEADER,
                &x_goog_request_params,
            )
            .await
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
