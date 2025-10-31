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

use super::object_descriptor::ObjectDescriptor;
use crate::google::storage::v2::{
    BidiReadObjectRequest, BidiReadObjectResponse, BidiReadObjectSpec,
};
use crate::request_options::RequestOptions;
use crate::{Error, Result};
use gaxi::grpc::Client as GrpcClient;
use tokio::sync::mpsc::Sender;

pub struct OpenObject {
    spec: BidiReadObjectSpec,
    options: RequestOptions,
    client: GrpcClient,
}

impl OpenObject {
    pub(crate) fn new(
        bucket: String,
        object: String,
        client: GrpcClient,
        options: RequestOptions,
    ) -> Self {
        let spec = BidiReadObjectSpec {
            bucket,
            object,
            ..BidiReadObjectSpec::default()
        };
        Self {
            spec,
            options,
            client,
        }
    }

    pub async fn send(mut self) -> Result<ObjectDescriptor> {
        println!("DEBUG DEBUG - send() spec={:?}", &self.spec);
        use gaxi::prost::FromProto;

        let request = BidiReadObjectRequest {
            read_object_spec: Some(self.spec.clone()),
            ..BidiReadObjectRequest::default()
        };
        let (tx, response) = self.connect_stream(request).await?;
        // TODO(coryan) - preserve the metadata for debugging.
        let (_metadata, mut stream, _) = response.into_parts();

        // TODO(coryan) - handle redirect errors.
        // If the start is None, then the stream closed successfully without any data. That is really an error.
        let Some(start) = stream.message().await.map_err(Error::io)? else {
            return Err(Error::io("bidi_read_object stream closed before start"));
        };
        let metadata = start
            .metadata
            .map(FromProto::cnv)
            .transpose()
            .map_err(Error::deser)?
            .ok_or_else(|| Error::deser("bidi_read_object is missing the object metadata value"))?;

        let handle = start.read_handle;
        println!("DEBUG DEBUG - handle = {handle:?}");
        let transport = super::ObjectDescriptorTransport::new(metadata, self, tx, stream);

        Ok(ObjectDescriptor::new(transport))
    }

    async fn connect_stream(
        &self,
        request: BidiReadObjectRequest,
    ) -> Result<(
        Sender<BidiReadObjectRequest>,
        tonic::Response<tonic::Streaming<BidiReadObjectResponse>>,
    )> {
        println!("DEBUG DEBUG - sending {request:?}");
        let (tx, rx) = tokio::sync::mpsc::channel::<BidiReadObjectRequest>(100);
        tx.send(request).await.map_err(Error::io)?;

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
        let bucket_name = request
            .read_object_spec
            .map(|s| s.bucket.as_str())
            .unwrap_or_default();
        let x_goog_request_params = format!("bucket={bucket_name}",);
        println!("DEBUG DEBUG - streaming_read({bucket_name}) - {x_goog_request_params}");
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        let response = self
            .client
            .bidi_stream::<BidiReadObjectRequest, BidiReadObjectResponse>(
                extensions,
                path,
                stream,
                options,
                &super::info::X_GOOG_API_CLIENT_HEADER,
                &x_goog_request_params,
            )
            .await?;
        println!("DEBUG DEBUG - received response {response:?}");

        // TODO(coryan) - preserve metadata for debugging.
        let (_metadata, stream, _extensions) = response.into_parts();
        println!("DEBUG DEBUG - metadata {_metadata:?},, stream={stream:?}");
        Ok((tx, stream))
    }
}

impl super::Reconnect for OpenObject {
    async fn connect(
        &self,
        ranges: Vec<ProtoRange>,
    ) -> Result<(
        Sender<BidiReadObjectRequest>,
        tonic::Response<tonic::Streaming<BidiReadObjectResponse>>,
    )> {
        let request = BidiReadObjectRequest {
            read_object_spec: Some(self.spec.clone()),
            read_ranges: ranges,
            ..BidiReadObjectRequest::default()
        };
        self.connect_stream(request).await
    }
}
