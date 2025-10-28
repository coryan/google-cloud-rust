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

use crate::google::storage::v2::BidiReadObjectResponse;
use crate::model::Object;
use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{Receiver, Sender};

/// Represents an open object in Cloud Storage.
#[derive(Clone, Debug)]
pub struct ObjectDescriptor {
    object: Object,
    inner: Arc<Mutex<ObjectDescriptorInner>>,
}

impl ObjectDescriptor {
    pub(crate) fn new(object: Object) -> Self {
        Self {
            object,
            inner: Arc::new(Mutex::new(ObjectDescriptorInner::new())),
        }
    }

    pub fn object(&self) -> &Object {
        &self.object
    }

    pub async fn read_range(&mut self, range: crate::model_ext::ReadRange) -> RangeResponse {
        self.inner
            .clone()
            .lock()
            .expect("never poisoned")
            .read_range(range)
            .await
    }

    async fn background(
        &self,
        mut source: tonic::codec::Streaming<BidiReadObjectResponse>,
    ) -> Result<()> {
        let inner = self.inner.clone();
        while let Some(m) = source.message().await.transpose() {
            match m {
                Err(e) => {
                    for (_, pending) in inner.lock().expect("never poisoned").ranges.drain() {
                        let _ = pending
                            .tx
                            .send(Err(Error::io(format!("TODO - pass on the error: {e:?}"))))
                            .await;
                    }
                }
                Ok(r) => {
                    for d in r.object_data_ranges {
                        let (Some(range), Some(data)) = (d.read_range, d.checksummed_data) else {
                            continue;
                        };
                        let id = range.read_id;
                        let mut guard = inner.lock().expect("never poisoned");

                        let payload = Payload {
                            data: data.content,
                            offset: range.read_offset,
                        };
                        if d.range_end {
                            if let Some(p) = guard.ranges.remove(&id) {
                                let _ = p.tx.send(Ok(payload)).await;
                            }
                        } else {
                            if let Some(p) = guard.ranges.get(&id) {
                                let _ = p.tx.send(Ok(payload)).await;
                            }
                        };
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ObjectDescriptorInner {
    read_id: i64,
    ranges: HashMap<i64, PendingRangeRead>,
}

impl ObjectDescriptorInner {
    pub fn new() -> Self {
        Self {
            read_id: 0,
            ranges: HashMap::new(),
        }
    }

    pub async fn read_range(&mut self, range: crate::model_ext::ReadRange) -> RangeResponse {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let id = self.read_id;
        let pending = PendingRangeRead { range, tx };
        self.read_id += 1;
        self.ranges.insert(id, pending);
        RangeResponse { rx }
    }
}

pub struct RangeResponse {
    rx: Receiver<Result<Payload>>,
}

impl RangeResponse {
    pub async fn next(&mut self) -> Option<Result<Payload>> {
        self.rx.recv().await
    }
}

pub struct Payload {
    pub data: bytes::Bytes,
    pub offset: i64,
}

#[derive(Debug)]
struct PendingRangeRead {
    range: crate::model_ext::ReadRange,
    tx: Sender<Result<Payload>>,
}
