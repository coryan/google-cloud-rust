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

use crate::error::ReadError;
use crate::google::storage::v2::{
    BidiReadObjectRequest, BidiReadObjectResponse, BidiReadObjectSpec,
};
use crate::model::Object;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::Receiver;

#[async_trait::async_trait]
trait GrpcStreamMaker {
    async fn new(
        client: &gaxi::grpc::Client,
        request: BidiReadObjectSpec,
    ) -> crate::Result<(
        Receiver<BidiReadObjectRequest>,
        crate::Result<tonic::Response<tonic::Streaming<BidiReadObjectResponse>>>,
    )>;
}

pub struct ObjectDescriptorTransport {
    object: Object,
    ranges: std::collections::HashMap<i32, PendingRange>,
    next_range_id: i32,
}

struct PendingRange {
    offset: i64,
    remaining: i64,
    sender: Sender<Result<bytes::Bytes, ReadError>>,
}
