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

use crate::model::Object;
use crate::model_ext::ReadRange;
use tokio::sync::mpsc::Receiver;

pub struct ObjectDescriptor {
    inner: Box<dyn stub::ObjectDescriptor>,
}

impl ObjectDescriptor {
    pub fn object(&self) -> Option<&Object> {
        self.inner.object()
    }

    pub async fn read_range(&self, range: ReadRange) -> super::RangeReader {
        self.inner.read_range(range).await
    }
}

pub mod stub {
    pub trait ObjectDescriptor {
        pub fn object(&self) -> Option<&Object>;
        pub fn read_range(&self, range: ReadRange) -> impl Future<Output = super::RangeReader>;
    }
}

pub struct ObjectDescriptorTransport {
    object: Option<Object>,
}

#[cfg(test)]
mod tests {
    use super::*;
}
