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

use tokio_stream::StreamExt;

use super::object::ObjectDescriptor;
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
        let (tx, response) = self
            .stub
            .streaming_read(&self.spec.bucket, gax::options::RequestOptions::default())
            .await?;
        let request = BidiReadObjectRequest {
            read_object_spec: Some(self.spec.clone()),
            ..BidiReadObjectRequest::default()
        };
        tx.send(request).await.map_err(Error::io)?;
        // TODO(coryan) - preserve metadata for debugging.
        let (_metadata, mut stream, _extensions) = response.into_parts();
        // TODO(coryan) - handle redirect errors.
        // If the start is None, then the stream closed successfully without any data. That is really an error.
        let Some(start) = stream.next().await.transpose().map_err(Error::io)? else {
            return Err(Error::io("bidi_read_object stream closed before start"));
        };
        use gaxi::prost::FromProto;
        let metadata = start
            .metadata
            .map(crate::google::storage::v2::Object::cnv)
            .transpose()
            .map_err(Error::deser)?
            .ok_or_else(|| Error::deser("bidi_read_object is missing the object metadata value"))?;
        Ok(ObjectDescriptor::new(metadata))
    }
}
