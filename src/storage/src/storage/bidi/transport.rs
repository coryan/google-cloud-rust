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

use super::pending_range::PendingRange;
use crate::error::ReadError;
use crate::google::storage::v2::{BidiReadObjectRequest, BidiReadObjectResponse};
use crate::google::storage::v2::{ObjectRangeData, ReadRange as ProtoRange};
use crate::model::Object;
use crate::model_ext::ReadRange;
use crate::storage::bidi::RangeReader;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

type ReadResult<T> = std::result::Result<T, ReadError>;

#[async_trait::async_trait]
trait Reconnect {
    async fn connect(
        &self,
        request: Vec<ProtoRange>,
    ) -> crate::Result<(
        Sender<BidiReadObjectRequest>,
        tonic::Response<tonic::Streaming<BidiReadObjectResponse>>,
    )>;
}

pub struct ObjectDescriptorTransport {
    object: Object,
    state: Arc<Mutex<TransportState>>,
    background: JoinHandle<()>,
}

impl ObjectDescriptorTransport {
    fn new<T>(
        object: Object,
        reconnect: T,
        tx: Sender<BidiReadObjectRequest>,
        stream: tonic::Streaming<BidiReadObjectResponse>,
    ) -> Self
    where
        T: Reconnect + Send + Sync + 'static,
    {
        let state = TransportState {
            ranges: HashMap::new(),
            next_range_id: 0_i64,
            tx,
            stream,
        };
        let state = Arc::new(Mutex::new(state));
        let bg = state.clone();
        let handle = tokio::spawn(async move { Self::run_background(bg, reconnect).await });
        Self {
            object,
            state,
            background: handle,
        }
    }

    async fn read_range(&mut self, range: ReadRange) -> RangeReader {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let range = PendingRange::new(tx, range, self.object.size);
        let mut guard = self.state.lock().await;
        guard.insert_range(range).await;
        drop(guard);
        RangeReader::new(rx)
    }

    async fn run_background<T: Reconnect + Send + Sync + 'static>(
        state: Arc<Mutex<TransportState>>,
        reconnect: T,
    ) {
        while let Some(message) = state.clone().lock().await.next_message(&reconnect).await {
            let pending = message
                .object_data_ranges
                .into_iter()
                .map(|r| Self::handle_response(state.clone(), r))
                .collect::<Vec<_>>();
            let _ = futures::future::join_all(pending).await;
        }
    }

    async fn handle_response(
        state: Arc<Mutex<TransportState>>,
        response: ObjectRangeData,
    ) -> ReadResult<()> {
        let mut guard = state.lock().await;
        guard.handle_response(response).await
    }
}

struct TransportState {
    ranges: HashMap<i64, PendingRange>,
    next_range_id: i64,
    tx: Sender<BidiReadObjectRequest>,
    stream: tonic::Streaming<BidiReadObjectResponse>,
}

impl TransportState {
    async fn next_message<T: Reconnect>(
        &mut self,
        reconnect: &T,
    ) -> Option<BidiReadObjectResponse> {
        loop {
            let message = self.stream.message().await;
            match message {
                Ok(Some(m)) => return Some(m),
                // The underlying stream was closed successfully. That only
                // happens if the application drops the object descriptor and no
                // longer wants to read data.
                Ok(None) => return None,
                Err(e) => {
                    // TODO: query the resume policy
                    println!("error reading from bi-di stream: {e:?}");
                }
            };
            let ranges: Vec<_> = self.ranges.iter().map(|(id, r)| r.as_proto(*id)).collect();
            match reconnect.connect(ranges).await {
                Err(e) => {
                    let error = Arc::new(e);
                    let closing: Vec<_> = self
                        .ranges
                        .iter_mut()
                        .map(|(_, pending)| pending.interrupted(error.clone()))
                        .collect();
                    let _ = futures::future::join_all(closing).await;
                    return None;
                }
                Ok((tx, response)) => {
                    // TODO: maybe do something with the metadata, like save the x-goog-uploader-id ?
                    let (_, stream, _) = response.into_parts();
                    self.tx = tx;
                    self.stream = stream;
                }
            };
        }
    }

    async fn insert_range(&mut self, range: PendingRange) {
        let id = self.next_range_id;
        self.next_range_id += 1;

        let request = range.as_proto(id);
        self.ranges.insert(id, range);
        let request = BidiReadObjectRequest {
            read_ranges: vec![request],
            ..BidiReadObjectRequest::default()
        };
        // Any errors here are recovered by the main background loop.
        if let Err(e) = self.tx.send(request).await {
            tracing::error!("error sending read range request: {e:?}");
        }
    }

    async fn handle_response(
        &mut self,
        response: crate::google::storage::v2::ObjectRangeData,
    ) -> ReadResult<()> {
        let range = response
            .read_range
            .ok_or(ReadError::MissingRangeInBidiResponse)?;
        if response.range_end {
            let mut pending = self
                .ranges
                .remove(&range.read_id)
                .ok_or(ReadError::UnknownRange(range.read_id))?;
            pending.handle_data(range, response.checksummed_data).await
        } else {
            let pending = self
                .ranges
                .get_mut(&range.read_id)
                .ok_or(ReadError::UnknownRange(range.read_id))?;
            pending.handle_data(range, response.checksummed_data).await
        }
    }
}
