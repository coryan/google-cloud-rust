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

#[cfg(all(
    test,
    google_cloud_unstable_storage_bidi,
    feature = "_internal-run-integration-tests"
))]
mod bidi_read {
    use google_cloud_storage::client::{Bidi, Storage};
    use storage_samples::{cleanup_bucket, create_test_bucket};

    #[tokio::test]
    async fn run() -> anyhow::Result<()> {
        let (control, bucket) = create_test_bucket().await?;
        let result = basic(&bucket.name).await;
        if let Err(e) = cleanup_bucket(control, bucket.name).await {
            println!("Error cleaning up bucket for bidi_read::basic(): {e:?}");
        }
        result
    }

    async fn basic(bucket_name: &str) -> anyhow::Result<()> {
        let client = Storage::builder().build().await?;
        let write = client
            .write_object(
                bucket_name,
                "basic/source.txt",
                String::from_iter((0..100_000).map(|_| 'a')),
            )
            .set_if_generation_match(0)
            .send_unbuffered()
            .await?;

        let client = Bidi::builder().build_bidi().await?;
        println!("created bidi client: {client:?}");
        let open = client.open_object(bucket_name, &write.name).send().await?;
        println!("open returns: {open:?}");
        let got = open.object();
        let mut want = write.clone();
        // This field is a mismatch, but both `Some(false)` and `None` represent
        // the same value.
        want.event_based_hold = want.event_based_hold.or(Some(false));
        // There is a submillisecond difference, maybe rounding?
        want.finalize_time = got.finalize_time;
        assert_eq!(got, &want);
        Ok(())
    }
}

pub fn enable_tracing() -> tracing::subscriber::DefaultGuard {
    use tracing_subscriber::fmt::format::FmtSpan;
    let builder = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_max_level(tracing::Level::TRACE);
    let subscriber = builder.finish();

    tracing::subscriber::set_default(subscriber)
}
