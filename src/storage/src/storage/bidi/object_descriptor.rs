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

use super::RangeReader;
use crate::error::ReadError;
use crate::model::Object;
use crate::model_ext::ReadRange;
use tokio::sync::mpsc::Sender;

pub struct ObjectDescriptor {
    inner: Box<dyn stub::dynamic::ObjectDescriptor>,
}

impl ObjectDescriptor {
    pub fn object(&self) -> &Object {
        self.inner.object()
    }

    pub async fn read_range(&self, range: ReadRange) -> RangeReader {
        self.inner.read_range(range).await
    }
}

pub mod stub {
    use super::{Object, RangeReader, ReadRange};

    pub trait ObjectDescriptor: std::fmt::Debug + Send + Sync {
        fn object(&self) -> &Object;
        fn read_range(&self, range: ReadRange) -> impl Future<Output = RangeReader> + Send + Sync;
    }

    pub(crate) mod dynamic {
        use super::{Object, RangeReader, ReadRange};

        #[async_trait::async_trait]
        pub trait ObjectDescriptor: std::fmt::Debug + Send + Sync {
            fn object(&self) -> &Object;
            async fn read_range(&self, range: ReadRange) -> RangeReader;
        }

        #[async_trait::async_trait]
        impl<T: super::ObjectDescriptor> ObjectDescriptor for T {
            fn object(&self) -> &Object {
                T::object(self)
            }

            async fn read_range(&self, range: ReadRange) -> RangeReader {
                T::read_range(self, range).await
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
}
