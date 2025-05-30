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

use super::Result;
use super::backoff_policy::BackoffPolicy;
use super::loop_state::LoopState;
use super::retry_policy::RetryPolicy;
use super::retry_throttler::RetryThrottler;
use std::sync::{Arc, Mutex};

/// Runs the retry loop for a given function.
///
/// This functions calls an inner function as long as (1) the retry policy has
/// not expired, (2) the inner function has not returned a successful request,
/// and (3) the retry throttler allows more calls.
///
/// In between calls the function waits the amount of time prescribed by the
/// backoff policy, using `sleep` to implement any sleep.
pub async fn retry_loop<F, S, Response>(
    inner: F,
    sleep: S,
    idempotent: bool,
    retry_throttler: Arc<Mutex<dyn RetryThrottler>>,
    retry_policy: Arc<dyn RetryPolicy>,
    backoff_policy: Arc<dyn BackoffPolicy>,
) -> Result<Response>
where
    F: AsyncFn(Option<std::time::Duration>) -> Result<Response> + Send,
    S: AsyncFn(std::time::Duration) -> () + Send,
{
    let loop_start = tokio::time::Instant::now().into_std();
    let mut attempt_count = 0;
    loop {
        let remaining_time = retry_policy.remaining_time(loop_start, attempt_count);
        let throttle = if attempt_count == 0 {
            false
        } else {
            let t = retry_throttler
                .lock()
                .expect("retry throttler lock is poisoned");
            t.throttle_retry_attempt()
        };
        if throttle {
            // This counts as an error for the purposes of the retry policy.
            if let Some(error) = retry_policy.on_throttle(loop_start, attempt_count) {
                return Err(error);
            }
            let delay = backoff_policy.on_failure(loop_start, attempt_count);
            sleep(delay).await;
            continue;
        }
        attempt_count += 1;
        match inner(remaining_time).await {
            Ok(r) => {
                retry_throttler
                    .lock()
                    .expect("retry throttler lock is poisoned")
                    .on_success();
                return Ok(r);
            }
            Err(e) => {
                let flow = retry_policy.on_error(loop_start, attempt_count, idempotent, e);
                let delay = backoff_policy.on_failure(loop_start, attempt_count);
                retry_throttler
                    .lock()
                    .expect("retry throttler lock is poisoned")
                    .on_retry_failure(&flow);
                on_error(&sleep, flow, delay).await?;
            }
        };
    }
}

async fn on_error<B>(
    backoff: &B,
    retry_flow: LoopState,
    backoff_delay: std::time::Duration,
) -> Result<()>
where
    B: AsyncFn(std::time::Duration) -> (),
{
    match retry_flow {
        LoopState::Permanent(e) | LoopState::Exhausted(e) => Err(e),
        LoopState::Continue(_e) => {
            backoff(backoff_delay).await;
            Ok(())
        }
    }
}

/// A helper to compute the time remaining in a retry loop, given the attempt
/// timeout and the overall timeout.
pub fn effective_timeout(
    options: &crate::options::RequestOptions,
    remaining_time: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (options.attempt_timeout(), remaining_time) {
        (None, None) => None,
        (None, Some(t)) => Some(t),
        (Some(t), None) => Some(*t),
        (Some(a), Some(r)) => Some(*std::cmp::min(a, &r)),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::{Error, rpc::Code, rpc::Status};
    use std::time::Duration;
    use test_case::test_case;

    #[test_case(None, None, None)]
    #[test_case(Some(Duration::from_secs(4)), Some(Duration::from_secs(4)), None)]
    #[test_case(Some(Duration::from_secs(4)), None, Some(Duration::from_secs(4)))]
    #[test_case(
        Some(Duration::from_secs(2)),
        Some(Duration::from_secs(2)),
        Some(Duration::from_secs(4))
    )]
    #[test_case(
        Some(Duration::from_secs(2)),
        Some(Duration::from_secs(4)),
        Some(Duration::from_secs(2))
    )]
    fn effective_timeouts(
        want: Option<Duration>,
        remaining: Option<Duration>,
        request: Option<Duration>,
    ) {
        let options = crate::options::RequestOptions::default();
        let options = request.into_iter().fold(options, |mut o, t| {
            o.set_attempt_timeout(t);
            o
        });
        let got = effective_timeout(&options, remaining);
        assert_eq!(want, got);
    }

    #[tokio::test]
    async fn immediate_success() -> anyhow::Result<()> {
        // This test simulates a server immediate returning a successful
        // response.
        let mut call = MockCall::new();
        call.expect_call().once().returning(|_| success());
        let inner = async move |d| call.call(d);

        let mut throttler = MockRetryThrottler::new();
        throttler.expect_on_success().once().return_const(());
        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .once()
            .return_const(None);
        let backoff_policy = MockBackoffPolicy::new();
        let sleep = MockSleep::new();

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await?;
        assert_eq!(response, "success");
        Ok(())
    }

    #[tokio::test]
    async fn immediate_failure() -> anyhow::Result<()> {
        // This test simulates a server responding with an immediate and
        // permanent error.
        let mut call = MockCall::new();
        call.expect_call().once().returning(|_| permanent());
        let inner = async move |d| call.call(d);

        let mut throttler = MockRetryThrottler::new();
        throttler.expect_on_retry_failure().once().return_const(());
        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .once()
            .return_const(None);
        retry_policy
            .expect_on_error()
            .once()
            .returning(|_, _, _, e| LoopState::Permanent(e));
        let mut backoff_policy = MockBackoffPolicy::new();
        backoff_policy
            .expect_on_failure()
            .once()
            .return_const(Duration::from_secs(0));
        let sleep = MockSleep::new();

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(response.is_err(), "{response:?}");
        Ok(())
    }

    #[test_case(true)]
    #[test_case(false)]
    #[tokio::test]
    async fn retry_success(expected_idempotency: bool) -> anyhow::Result<()> {
        // This test simulates a server responding with two transient errors and
        // then with a successful response.
        let mut call_seq = mockall::Sequence::new();
        let mut call = MockCall::new();
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .withf(|got| got == &Some(Duration::from_secs(3)))
            .returning(|_| transient());
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .withf(|got| got == &Some(Duration::from_secs(2)))
            .returning(|_| transient());
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .withf(|got| got == &Some(Duration::from_secs(1)))
            .returning(|_| success());
        let inner = async move |d| call.call(d);

        let mut throttler_seq = mockall::Sequence::new();
        let mut throttler = MockRetryThrottler::new();
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(false);
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(false);
        throttler
            .expect_on_success()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());

        // Take the opportunity to verify the right values are provided to the
        // backoff policy and the remaining time.
        let mut retry_seq = mockall::Sequence::new();
        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .once()
            .in_sequence(&mut retry_seq)
            .return_const(Some(Duration::from_secs(3)));
        retry_policy
            .expect_remaining_time()
            .once()
            .in_sequence(&mut retry_seq)
            .return_const(Some(Duration::from_secs(2)));
        retry_policy
            .expect_remaining_time()
            .once()
            .in_sequence(&mut retry_seq)
            .return_const(Some(Duration::from_secs(1)));
        retry_policy
            .expect_on_error()
            .times(2)
            .withf(move |_, _, idempotent, _| idempotent == &expected_idempotency)
            .returning(|_, _, _, e| LoopState::Continue(e));

        let mut backoff_seq = mockall::Sequence::new();
        let mut backoff_policy = MockBackoffPolicy::new();
        let mut sleep_seq = mockall::Sequence::new();
        let mut sleep = MockSleep::new();

        for d in 1..=2 {
            backoff_policy
                .expect_on_failure()
                .once()
                .in_sequence(&mut backoff_seq)
                .return_const(Duration::from_millis(d));
            sleep
                .expect_sleep()
                .once()
                .in_sequence(&mut sleep_seq)
                .withf(move |got| got == &Duration::from_millis(d))
                .returning(|_| Box::pin(async {}));
        }

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            expected_idempotency,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(matches!(&response, Ok(s) if s == "success"), "{response:?}");
        Ok(())
    }

    #[tokio::test]
    async fn too_many_transients() -> anyhow::Result<()> {
        // This test simulates a server responding with two transient errors
        // and the retry policy stops after the second attempt.
        const ERRORS: usize = 2;
        let mut call_seq = mockall::Sequence::new();
        let mut call = MockCall::new();
        for _ in 0..ERRORS {
            call.expect_call()
                .once()
                .withf(|d| d.is_none())
                .in_sequence(&mut call_seq)
                .returning(|_| transient());
        }
        let inner = async move |d| call.call(d);

        let mut throttler_seq = mockall::Sequence::new();
        let mut throttler = MockRetryThrottler::new();
        for _ in 0..ERRORS - 1 {
            throttler
                .expect_on_retry_failure()
                .once()
                .in_sequence(&mut throttler_seq)
                .return_const(());
            throttler
                .expect_throttle_retry_attempt()
                .once()
                .in_sequence(&mut throttler_seq)
                .return_const(false);
        }
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());

        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .times(ERRORS)
            .return_const(None);
        let mut retry_seq = mockall::Sequence::new();
        retry_policy
            .expect_on_error()
            .times(ERRORS - 1)
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Continue(e));
        retry_policy
            .expect_on_error()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Exhausted(e));
        let mut backoff_policy = MockBackoffPolicy::new();
        backoff_policy
            .expect_on_failure()
            .times(ERRORS)
            .return_const(Duration::from_secs(0));

        let mut sleep = MockSleep::new();
        sleep
            .expect_sleep()
            .times(ERRORS - 1)
            .returning(|_| Box::pin(async {}));

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(response.is_err(), "{response:?}");
        Ok(())
    }

    #[tokio::test]
    async fn transient_then_permanent() -> anyhow::Result<()> {
        // This test simulates a server responding with a transient errors
        // and then a permanent error. The retry loop should stop on the second
        // error.
        let mut call_seq = mockall::Sequence::new();
        let mut call = MockCall::new();
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .returning(|_| transient());
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .returning(|_| permanent());
        let inner = async move |d| call.call(d);

        let mut throttler_seq = mockall::Sequence::new();
        let mut throttler = MockRetryThrottler::new();
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(false);
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());

        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .times(2)
            .return_const(None);
        let mut retry_seq = mockall::Sequence::new();
        retry_policy
            .expect_on_error()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Continue(e));
        retry_policy
            .expect_on_error()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Permanent(e));
        let mut backoff_policy = MockBackoffPolicy::new();
        backoff_policy
            .expect_on_failure()
            .times(2)
            .return_const(Duration::from_secs(0));

        let mut sleep = MockSleep::new();
        sleep
            .expect_sleep()
            .once()
            .returning(|_| Box::pin(async {}));

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(response.is_err(), "{response:?}");
        Ok(())
    }

    #[tokio::test]
    async fn throttle_then_success() -> anyhow::Result<()> {
        // This test simulates a server responding with two transient errors and
        // then with a successful response.
        let mut call_seq = mockall::Sequence::new();
        let mut call = MockCall::new();
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .returning(|_| transient());
        call.expect_call()
            .once()
            .in_sequence(&mut call_seq)
            .returning(|_| success());
        let inner = async move |d| call.call(d);

        let mut throttler_seq = mockall::Sequence::new();
        let mut throttler = MockRetryThrottler::new();
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());
        // Skip one request ..
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(true);
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(false);
        throttler
            .expect_on_success()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());

        let mut retry_seq = mockall::Sequence::new();
        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .times(3)
            .return_const(None);
        retry_policy
            .expect_on_error()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Continue(e));
        retry_policy
            .expect_on_throttle()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _| None);

        let mut backoff_policy = MockBackoffPolicy::new();
        backoff_policy
            .expect_on_failure()
            .times(2)
            .return_const(Duration::from_secs(0));

        let mut sleep = MockSleep::new();
        sleep
            .expect_sleep()
            .times(2)
            .returning(|_| Box::pin(async {}));

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(matches!(&response, Ok(s) if s == "success"), "{response:?}");
        Ok(())
    }

    #[tokio::test]
    async fn throttle_and_retry_policy_stops_loop() -> anyhow::Result<()> {
        // This test simulates a server responding with a transient error, and
        // the retry attempt is throttled, and then the retry loop stops the
        // loop.
        let mut call = MockCall::new();
        call.expect_call().once().returning(|_| transient());
        let inner = async move |d| call.call(d);

        let mut throttler_seq = mockall::Sequence::new();
        let mut throttler = MockRetryThrottler::new();
        throttler
            .expect_on_retry_failure()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(());
        // Skip one request ..
        throttler
            .expect_throttle_retry_attempt()
            .once()
            .in_sequence(&mut throttler_seq)
            .return_const(true);

        let mut retry_seq = mockall::Sequence::new();
        let mut retry_policy = MockRetryPolicy::new();
        retry_policy
            .expect_remaining_time()
            .times(2)
            .return_const(None);
        retry_policy
            .expect_on_error()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _, _, e| LoopState::Continue(e));
        retry_policy
            .expect_on_throttle()
            .once()
            .in_sequence(&mut retry_seq)
            .returning(|_, _| Some(Error::other("retry-policy-on-throttle")));

        let mut backoff_policy = MockBackoffPolicy::new();
        backoff_policy
            .expect_on_failure()
            .once()
            .return_const(Duration::from_secs(0));

        let mut sleep = MockSleep::new();
        sleep
            .expect_sleep()
            .once()
            .returning(|_| Box::pin(async {}));

        let backoff = async move |d| sleep.sleep(d).await;
        let response = retry_loop(
            inner,
            backoff,
            true,
            to_retry_throttler(throttler),
            to_retry_policy(retry_policy),
            to_backoff_policy(backoff_policy),
        )
        .await;
        assert!(
            matches!(&response, Err(e) if format!("{e}").contains("retry-policy-on-throttle")),
            "{response:?}"
        );
        Ok(())
    }

    fn success() -> Result<String> {
        Ok("success".into())
    }

    fn transient() -> Result<String> {
        let status = Status::default()
            .set_code(Code::Unavailable)
            .set_message("try-again");
        Err(Error::service(status))
    }

    fn permanent() -> Result<String> {
        let status = Status::default()
            .set_code(Code::PermissionDenied)
            .set_message("uh-oh");
        Err(Error::service(status))
    }

    fn to_retry_throttler(mock: MockRetryThrottler) -> Arc<Mutex<dyn RetryThrottler>> {
        Arc::new(Mutex::new(mock))
    }

    fn to_retry_policy(mock: MockRetryPolicy) -> Arc<dyn RetryPolicy> {
        Arc::new(mock)
    }

    fn to_backoff_policy(mock: MockBackoffPolicy) -> Arc<dyn BackoffPolicy> {
        Arc::new(mock)
    }

    trait Call {
        fn call(&self, d: Option<Duration>) -> Result<String>;
    }

    mockall::mock! {
        Call {}
        impl Call for Call {
            fn call(&self, d: Option<Duration>) -> Result<String>;
        }
    }

    trait Sleep {
        fn sleep(&self, d: std::time::Duration) -> impl Future<Output = ()>;
    }

    mockall::mock! {
        Sleep {}
        impl Sleep for Sleep {
            fn sleep(&self, d: std::time::Duration) -> impl Future<Output = ()> + Send;
        }
    }

    mockall::mock! {
        #[derive(Debug)]
        RetryPolicy {}
        impl RetryPolicy for RetryPolicy {
            fn on_error(&self, loop_start: std::time::Instant, attempt_count: u32, idempotent: bool, error: Error) -> LoopState;
            fn on_throttle(&self, loop_start: std::time::Instant, attempt_count: u32) -> Option<Error>;
            fn remaining_time(&self, loop_start: std::time::Instant, attempt_count: u32) -> Option<std::time::Duration>;
        }
        impl std::clone::Clone for RetryPolicy {
            fn clone(&self) -> Self;
        }
    }

    mockall::mock! {
        #[derive(Debug)]
        BackoffPolicy {}
        impl BackoffPolicy for BackoffPolicy {
            fn on_failure(&self, loop_start: std::time::Instant, attempt_count: u32) -> std::time::Duration;
        }
    }

    mockall::mock! {
        #[derive(Debug)]
        RetryThrottler {}
        impl RetryThrottler for RetryThrottler {
            fn throttle_retry_attempt(&self) -> bool;
            fn on_retry_failure(&mut self, error: &LoopState);
            fn on_success(&mut self);
        }
    }
}
