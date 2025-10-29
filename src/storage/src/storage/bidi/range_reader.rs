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
use tokio::sync::mpsc::Receiver;

/// Read the data from a [ObjectDescriptor][super::ObjectDescriptor] range.
///
/// This type is used to stream an open object descriptor range.
pub struct RangeReader {
    inner: Receiver<Result<bytes::Bytes, ReadError>>,
}

impl RangeReader {
    // Get the next chunk of data.
    pub async fn next(&mut self) -> Option<Result<bytes::Bytes, ReadError>> {
        self.inner.recv().await
    }

    /// Create a new instance.
    ///
    /// This constructor is useful when mocking `ObjectDescriptor`.
    pub fn new(inner: Receiver<Result<bytes::Bytes, ReadError>>) -> Self {
        Self { inner }
    }
}
