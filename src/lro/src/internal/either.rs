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

use crate::{BasePoller, sealed};

/// Combine two different `BasePoller` types into a single type.
#[derive(Clone, Debug)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> sealed::Poller for Either<A, B> {}

impl<A, B, ResponseType, MetadataType> BasePoller<ResponseType, MetadataType> for Either<A, B>
where
    A: BasePoller<ResponseType, MetadataType>,
    B: BasePoller<ResponseType, MetadataType>,
{
    async fn poll(&mut self) -> Option<crate::PollingResult<ResponseType, MetadataType>> {
        match self {
            Self::Left(s) => s.poll().await,
            Self::Right(s) => s.poll().await,
        }
    }
    async fn sleep(&mut self, backoff: std::time::Duration) {
        match self {
            Self::Left(s) => s.sleep(backoff).await,
            Self::Right(s) => s.sleep(backoff).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PollingResult;
    use google_cloud_wkt::{Duration, Timestamp};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type ResponseType = Duration;
    type MetadataType = Timestamp;

    struct PollerA(Arc<AtomicUsize>);
    impl sealed::Poller for PollerA {}
    impl BasePoller<ResponseType, MetadataType> for PollerA {
        async fn poll(&mut self) -> Option<PollingResult<ResponseType, MetadataType>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            None
        }
        async fn sleep(&mut self, _backoff: std::time::Duration) {
            self.0.fetch_add(10, Ordering::SeqCst);
        }
    }

    struct PollerB(Arc<AtomicUsize>);
    impl sealed::Poller for PollerB {}
    impl BasePoller<ResponseType, MetadataType> for PollerB {
        async fn poll(&mut self) -> Option<PollingResult<ResponseType, MetadataType>> {
            self.0.fetch_add(2, Ordering::SeqCst);
            None
        }
        async fn sleep(&mut self, _backoff: std::time::Duration) {
            self.0.fetch_add(20, Ordering::SeqCst);
        }
    }

    fn create_poller(
        use_a: bool,
        counter: Arc<AtomicUsize>,
    ) -> impl BasePoller<ResponseType, MetadataType> {
        if use_a {
            Either::Left(PollerA(counter))
        } else {
            Either::Right(PollerB(counter))
        }
    }

    #[tokio::test]
    async fn test_either_poller() {
        let counter_a = Arc::new(AtomicUsize::new(0));
        let mut poller_a = create_poller(true, Arc::clone(&counter_a));

        let counter_b = Arc::new(AtomicUsize::new(0));
        let mut poller_b = create_poller(false, Arc::clone(&counter_b));

        // Verify that Left delegates correctly
        poller_a.poll().await;
        assert_eq!(counter_a.load(Ordering::SeqCst), 1);
        poller_a.sleep(std::time::Duration::from_millis(1)).await;
        assert_eq!(counter_a.load(Ordering::SeqCst), 11);

        // Verify that Right delegates correctly
        poller_b.poll().await;
        assert_eq!(counter_b.load(Ordering::SeqCst), 2);
        poller_b.sleep(std::time::Duration::from_millis(1)).await;
        assert_eq!(counter_b.load(Ordering::SeqCst), 22);
    }
}
