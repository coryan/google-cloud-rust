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
use crate::google::storage::v2::{ChecksummedData, ReadRange as ProtoRange};
use crate::model_ext::ReadRange;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

type ReadResult<T> = std::result::Result<T, ReadError>;

pub(crate) struct PendingRange {
    offset: i64,
    limit: i64,
    sender: Sender<Result<bytes::Bytes, ReadError>>,
}

impl PendingRange {
    pub(super) fn new(
        sender: Sender<Result<bytes::Bytes, ReadError>>,
        range: ReadRange,
        size: i64,
    ) -> Self {
        let (offset, limit) = range.normalize(size);
        Self {
            sender,
            offset,
            limit,
        }
    }
    pub(super) async fn handle_data(
        &mut self,
        range: ProtoRange,
        data: Option<ChecksummedData>,
    ) -> ReadResult<()> {
        let Some(data) = data else {
            if self.limit == 0 {
                return Ok(());
            }
            return Err(ReadError::ShortRead(self.limit as u64));
        };
        if self.offset == range.read_offset {
            self.offset += range.read_length;
            self.limit -= range.read_length;
            self.limit = self.limit.clamp(0, i64::MAX);
            let _ = self.sender.send(Ok(data.content)).await;
            return Ok(());
        }
        Err(ReadError::OutOfOrderBidiResponse {
            got: range.read_offset,
            expected: range.read_offset,
        })
    }

    pub(super) async fn interrupted(&mut self, error: Arc<crate::Error>) {
        if let Err(e) = self
            .sender
            .send(Err(ReadError::UnrecoverableBidiReadInterrupt(error)))
            .await
        {
            tracing::error!("cannot notify sender about unrecoverable error: {e:?}");
        }
    }

    pub(super) fn as_proto(&self, id: i64) -> ProtoRange {
        ProtoRange {
            read_id: id,
            read_offset: self.offset,
            read_length: self.limit,
        }
    }
}
