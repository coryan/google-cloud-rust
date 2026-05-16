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
    use mockall::mock;

    type ResponseType = Duration;
    type MetadataType = Timestamp;

    mock! {
        PollerA {}
        impl sealed::Poller for PollerA {}
        impl BasePoller<ResponseType, MetadataType> for PollerA {
            async fn poll(&mut self) -> Option<PollingResult<ResponseType, MetadataType>>;
            async fn sleep(&mut self, backoff: std::time::Duration);
        }
    }
    mock! {
        PollerB {}
        impl sealed::Poller for PollerB {}
        impl BasePoller<ResponseType, MetadataType> for PollerB {
            async fn poll(&mut self) -> Option<PollingResult<ResponseType, MetadataType>>;
            async fn sleep(&mut self, backoff: std::time::Duration);
        }
    }

    #[tokio::test]
    async fn test_either_poller_left() {
        let mut mock = MockPollerA::new();
        mock.expect_poll().times(1).returning(|| None);
        mock.expect_sleep().times(1).returning(|_| ());

        let mut poller: Either<MockPollerA, MockPollerB> = Either::Left(mock);

        poller.poll().await;
        poller.sleep(std::time::Duration::from_millis(1)).await;
    }

    #[tokio::test]
    async fn test_either_poller_right() {
        let mut mock = MockPollerB::new();
        mock.expect_poll().times(1).returning(|| None);
        mock.expect_sleep().times(1).returning(|_| ());

        let mut poller: Either<MockPollerA, MockPollerB> = Either::Right(mock);

        poller.poll().await;
        poller.sleep(std::time::Duration::from_millis(1)).await;
    }

    #[tokio::test]
    async fn test_return_impl_base_poller() {
        fn factory(use_a: bool) -> impl BasePoller<ResponseType, MetadataType> {
            if use_a {
                Either::Left(MockPollerA::new())
            } else {
                Either::Right(MockPollerB::new())
            }
        }

        let _poller_a = factory(true);
        let _poller_b = factory(false);
    }
}
