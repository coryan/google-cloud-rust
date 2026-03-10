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

use super::{Error, RetryPolicy, RetryResult, RetryState, ThrottleResult};
use std::time::Duration;

#[derive(Debug)]
pub struct AttemptTimeout<P>
where
    P: RetryPolicy,
{
    inner: P,
}

impl<P> AttemptTimeout<P>
where
    P: RetryPolicy,
{
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P> RetryPolicy for AttemptTimeout<P>
where
    P: RetryPolicy,
{
    fn on_error(&self, state: &RetryState, error: Error) -> RetryResult {
        if error.is_timeout() {
            return RetryResult::Continue(error);
        }
        self.inner.on_error(state, error)
    }

    fn on_throttle(&self, state: &RetryState, error: Error) -> ThrottleResult {
        self.inner.on_throttle(state, error)
    }

    fn remaining_time(&self, state: &RetryState) -> Option<Duration> {
        self.inner.remaining_time(state)
    }
}
