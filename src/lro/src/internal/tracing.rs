// Copyright 2026 Google LLC
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

use crate::{BasePoller, sealed};
use tracing::{Instrument, info_span};

/// Decorate a poller with tracing information.
#[derive(Clone, Debug)]
pub struct Tracing<P> {
    inner: P,
}

impl<P> sealed::Poller for Tracing<P> {}

impl<P, ResponseType, MetadataType> BasePoller<ResponseType, MetadataType> for Tracing<P>
where
    P: BasePoller<ResponseType, MetadataType>,
{
    async fn poll(&mut self) -> Option<crate::PollingResult<ResponseType, MetadataType>> {
        let span = info_span!("LRO Poll");
        self.inner.poll().instrument(span).await
    }
    async fn sleep(&mut self, backoff: std::time::Duration) {
        let span = info_span!("LRO Sleep");
        self.inner.sleep(backoff).instrument(span).await
    }
}
