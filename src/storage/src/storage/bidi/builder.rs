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
use crate::google::storage::v2::BidiReadObjectRequest;
use crate::google::storage::v2::BidiReadObjectSpec;
use crate::request_options::RequestOptions;
use crate::{Error, Result};
use std::sync::Arc;

pub struct OpenObject<S> {
    spec: BidiReadObjectSpec,
    options: RequestOptions,
    stub: Arc<S>,
}

impl<S> OpenObject<S> {
    pub(crate) fn new(
        bucket: String,
        object: String,
        stub: Arc<S>,
        options: RequestOptions,
    ) -> Self {
        let spec = BidiReadObjectSpec {
            bucket,
            object,
            ..BidiReadObjectSpec::default()
        };
        Self {
            spec,
            stub,
            options,
        }
    }
}

impl<S> OpenObject<S>
where
    S: super::BidiStub,
{
    pub async fn send(self) -> Result<ObjectDescriptor> {
        println!("DEBUG DEBUG - send() spec={:?}", &self.spec);
        use gaxi::prost::FromProto;

        let (tx, rx) = tokio::sync::mpsc::channel::<BidiReadObjectRequest>(100);
        let request = BidiReadObjectRequest {
            read_object_spec: Some(self.spec.clone()),
            ..BidiReadObjectRequest::default()
        };
        println!("DEBUG DEBUG - sending {request:?}");
        tx.send(request).await.map_err(Error::io)?;

        let response = self
            .stub
            .streaming_read(
                &self.spec.bucket,
                rx,
                gax::options::RequestOptions::default(),
            )
            .await?;
        println!("DEBUG DEBUG - received response {response:?}");

        // TODO(coryan) - preserve metadata for debugging.
        let (_metadata, mut stream, _extensions) = response.into_parts();
        println!("DEBUG DEBUG - metadata {_metadata:?},, stream={stream:?}");
        // TODO(coryan) - handle redirect errors.
        // If the start is None, then the stream closed successfully without any data. That is really an error.
        let Some(start) = stream.message().await.map_err(Error::io)? else {
            return Err(Error::io("bidi_read_object stream closed before start"));
        };
        drop(tx);
        let metadata = start
            .metadata
            .map(FromProto::cnv)
            .transpose()
            .map_err(Error::deser)?
            .ok_or_else(|| Error::deser("bidi_read_object is missing the object metadata value"))?;

        let handle = start.read_handle;
        println!("DEBUG DEBUG - handle = {handle:?}");
        Ok(ObjectDescriptor::new(metadata))
    }
}
