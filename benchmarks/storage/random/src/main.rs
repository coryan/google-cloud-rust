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

//! Benchmark random-reads using the Cloud Storage client library fo Rust.

mod args;
mod dataset;
mod names;
mod sample;

use anyhow::Result;
use args::Args;
use clap::Parser;
use google_cloud_auth::credentials::{Builder as CredentialsBuilder, Credentials};
use rand::seq::IteratorRandom;
use sample::{Protocol, Sample};
use std::time::Instant;
use tokio::sync::mpsc;

const DESCRIPTION: &str = concat!(
    "This benchmark repeatedly reads ranges from a set of Cloud Storage objects.",
    " In each iteration of the benchmark the number of concurrent ranges,",
    " the size of the ranges, and the location of the ranges is selected at random.",
    " The API used for the download is also selected at random.",
    " The benchmark runs multiple tasks concurrently, all running identical loops."
);

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    args.validate()?;
    let _guard = enable_tracing(&args);

    let credentials = CredentialsBuilder::default().build()?;
    let objects = dataset::populate(&args, credentials.clone()).await?;
    let (tx, mut rx) = mpsc::channel(1024 * args.task_count);
    let test_start = Instant::now();
    let tasks = (0..args.task_count)
        .map(|task| {
            tokio::spawn(runner(
                task,
                test_start,
                credentials.clone(),
                tx.clone(),
                args.clone(),
                objects.clone(),
            ))
        })
        .collect::<Vec<_>>();
    drop(tx);
    println!("{}", Sample::HEADER);
    while let Some(sample) = rx.recv().await {
        println!("{}", sample.to_row());
    }

    tracing::info!("DONE");
    Ok(())
}

async fn runner(
    id: usize,
    test_start: Instant,
    credentials: Credentials,
    tx: mpsc::Sender<Sample>,
    args: Args,
    objects: Vec<String>,
) -> anyhow::Result<()> {
    let _guard = enable_tracing(&args);
    if id % 128 == 0 {
        tracing::info!("Task::run({})", id);
    }

    let bidi = google_cloud_storage::client::Storage::builder()
        .build_bidi()
        .await?;
    let client = google_cloud_storage::client::Storage::builder()
        .build_bidi()
        .await?;

    let mut rng = rand::rng();
    for iteration in 0..args.iterations {
        let protocol = [Protocol::Bidi, Protocol::Json]
            .into_iter()
            .choose(&mut rng)
            .expect("at lest one protocol selected");
    }

    Ok(())
}

fn enable_tracing(_args: &Args) -> tracing::dispatcher::DefaultGuard {
    use tracing_subscriber::fmt::format::FmtSpan;

    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_default(subscriber)
}
