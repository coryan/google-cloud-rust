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

//! An implementation of the W1R3 benchmark for Rust.
//!
//! The W1R3 benchmark repeatedly uploads an object, then downloads the object
//! 3 times, and then deletes the object. In each iteration of the benchmark the
//! size and name of the object is selected at random. The benchmark runs
//! multiple tasks concurrently.

use clap::Parser;
use google_cloud_gax::{options::RequestOptionsBuilder, paginator::ItemPaginator};
use google_cloud_storage::{Result as StorageResult, client::StorageControl, model::Object};
use rand::{Rng, distr::Uniform};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = enable_tracing();
    let args = Args::validated()?;
    tracing::info!("{args:?}");

    let control = StorageControl::builder().build().await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Object>(8 * args.max_batch_size);

    let test_start = Instant::now();
    let list = tokio::spawn(list(control.clone(), args.clone(), tx));

    println!("{}", Sample::HEADER);
    let size_gen = Uniform::new_inclusive(args.min_batch_size, args.max_batch_size).unwrap();
    for iteration in 0..args.min_sample_count {
        let size = rand::rng().sample(size_gen);
        let mut batch = Vec::new();
        if rx.recv_many(&mut batch, size).await == 0 {
            break;
        }
        let sample = delete_batch(&control, batch, size, test_start).await;
        sample.print(iteration);
    }
    tracing::info!("DONE - collected all required samples");
    drop(rx);
    match list.await {
        Err(e) => tracing::error!("error joining list task {e}"),
        Ok(Err(e)) => tracing::error!("error in list task {e}"),
        Ok(Ok(_)) => {}
    };

    Ok(())
}

struct Sample {
    target_size: usize,
    size: usize,
    elapsed: Duration,
    relative: Duration,
    errors: usize,
}

impl Sample {
    const HEADER: &str = "Iteration,TargetSize,BatchSize,ElapsedMicroseconds,RelativeMicroseconds,ErrorCount";

    fn print(&self, iteration: usize) {
        println!(
            "{iteration},{},{},{},{},{}",
            self.target_size,
            self.size,
            self.elapsed.as_micros(),
            self.relative.as_micros(),
            self.errors
        )
    }
}

async fn delete_batch(control: &StorageControl, batch: Vec<Object>, target_size: usize, test_start: Instant) -> Sample {
    let start = Instant::now();
    let relative = start - test_start;
    let results =
        futures::future::join_all(batch.into_iter().map(|o| delete_object(control, o))).await;
    let elapsed = Instant::now() - start;
    let size = results.len();
    let errors = results.into_iter().filter(Result::is_err).count();

    Sample {
        target_size,
        size,
        elapsed,
        relative,
        errors,
    }
}

async fn delete_object(control: &StorageControl, object: Object) -> StorageResult<()> {
    use google_cloud_gax::retry_policy::RetryPolicyExt;
    use google_cloud_storage::backoff_policy::default;
    use google_cloud_storage::retry_policy::RecommendedPolicy as Recommended;
    control
        .delete_object()
        .set_bucket(object.bucket)
        .set_object(object.name)
        .set_generation(object.generation)
        .with_idempotency(true)
        .with_retry_policy(Recommended.with_time_limit(Duration::from_secs(300)))
        .with_backoff_policy(default())
        .send()
        .await
}

async fn list(control: StorageControl, args: Args, tx: Sender<Object>) -> StorageResult<()> {
    let mut objects = control
        .list_objects()
        .set_parent(format!("projects/_/buckets/{}", args.bucket_name))
        .set_page_size(5_000)
        .by_item();
    while let Some(object) = objects.next().await.transpose()? {
        if let Err(_) = tx.send(object).await {
            break;
        }
    }
    Ok(())
}

fn enable_tracing() -> tracing::dispatcher::DefaultGuard {
    use tracing_subscriber::fmt::format::FmtSpan;
    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_default(subscriber)
}

#[derive(Clone, Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    bucket_name: String,

    #[arg(long, default_value_t = 1)]
    min_batch_size: usize,
    #[arg(long, default_value_t = 1024)]
    max_batch_size: usize,

    #[arg(long, default_value_t = 1)]
    task_count: i32,

    #[arg(long, default_value_t = 1)]
    min_sample_count: usize,
}

impl Args {
    fn validated() -> anyhow::Result<Self> {
        let args = Args::parse();
        match (args.min_batch_size, args.max_batch_size) {
            (0, _) => return Err(anyhow::Error::msg("min-batch-size must be > 0")),
            (min, max) if max < min => {
                return Err(anyhow::Error::msg(format!(
                    "min-batch-size ({min}) must be >= max-batch-size ({max})"
                )));
            }
            _ => {}
        };
        Ok(args)
    }
}
