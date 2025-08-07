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
use google_cloud_gax::options::RequestOptionsBuilder;
use google_cloud_gax::retry_policy::RetryPolicyExt;
use google_cloud_storage::backoff_policy;
use google_cloud_storage::client::{Storage, StorageControl};
use google_cloud_storage::retry_policy::RecommendedPolicy;
use rand::{
    Rng,
    distr::{Alphanumeric, Uniform},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::broadcast::{Sender, error::RecvError};

static WRITE_ERROR: AtomicU64 = AtomicU64::new(0);
static READ_ERROR: AtomicU64 = AtomicU64::new(0);
static DELETE_ERROR: AtomicU64 = AtomicU64::new(0);
static SEND_ERROR: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = enable_tracing();

    let args = Args::parse();
    tracing::info!("{args:?}");

    let client = Storage::builder().build().await?;
    let control = StorageControl::builder().build().await?;

    let (tx, mut rx) = tokio::sync::broadcast::channel(128);
    let test_start = Instant::now();
    let buffer = bytes::Bytes::from_owner(
        rand::rng()
            .sample_iter(Uniform::new_inclusive(u8::MIN, u8::MAX)?)
            .take(args.max_object_size as usize)
            .collect::<Vec<_>>(),
    );
    let tasks = (0..args.task_count)
        .map(|id| {
            let task = Task {
                id,
                start: test_start,
                client: client.clone(),
                control: control.clone(),
                tx: tx.clone(),
                buffer: buffer.clone(),
            };
            tokio::spawn(task.run(args.clone()))
        })
        .collect::<Vec<_>>();
    drop(tx);

    let id = uuid::Uuid::new_v4().to_string();
    println!("Experiment,{}", Sample::HEADER);
    let mut sample_count = 0_u64;
    loop {
        match rx.recv().await {
            Ok(sample) => {
                sample_count += 1;
                println!("{id},{}", sample.to_row())
            }
            Err(RecvError::Closed) => break,
            Err(RecvError::Lagged(_)) => continue,
        }
    }

    for t in tasks {
        t.await??;
    }
    [
        ("WRITE_ERROR", WRITE_ERROR.load(Ordering::Relaxed)),
        ("READ_ERROR", READ_ERROR.load(Ordering::Relaxed)),
        ("DELETE_ERROR", DELETE_ERROR.load(Ordering::Relaxed)),
        ("SEND_ERROR", SEND_ERROR.load(Ordering::Relaxed)),
        ("SAMPLE_COUNT", sample_count),
    ]
    .into_iter()
    .for_each(|(key, value)| {
        tracing::info!("{key} = {value}");
    });
    tracing::info!("DONE");
    Ok(())
}

#[derive(Clone)]
struct Task {
    client: Storage,
    control: StorageControl,
    start: Instant,
    buffer: bytes::Bytes,
    id: i32,
    tx: Sender<Sample>,
}

type ResultObject = google_cloud_storage::Result<google_cloud_storage::model::Object>;

impl Task {
    async fn run(self, args: Args) -> anyhow::Result<()> {
        let size = Uniform::new_inclusive(args.min_object_size, args.max_object_size)?;

        for iteration in 0..args.min_sample_count {
            let name = random_object_name();
            let size = rand::rng().sample(size) as usize;
            let (write_op, threshold) = if rand::rng().random_bool(0.5) {
                (Operation::Resumable, 0_usize)
            } else {
                (Operation::SingleShot, size)
            };

            let write_start = Instant::now();
            let ex = Experiment {
                task: self.id,
                relative: write_start - self.start,
                iteration,
                start: write_start,
                op: write_op,
                target_size: size,
                name: &name,
            };
            let upload = self
                .client
                .upload_object(
                    format!("projects/_/buckets/{}", &args.bucket_name),
                    &name,
                    self.buffer.slice(0..size),
                )
                .with_if_generation_match(0)
                .with_resumable_upload_threshold(threshold)
                .send_unbuffered()
                .await;
            let Ok(upload) = self.on_write(ex, upload).await else {
                continue;
            };
            for op in [Operation::Read0, Operation::Read1, Operation::Read2] {
                let read_start = Instant::now();
                let ex = Experiment {
                    task: self.id,
                    relative: read_start - self.start,
                    iteration,
                    start: read_start,
                    op,
                    target_size: size,
                    name: &upload.name,
                };
                let mut read = match self
                    .client
                    .read_object(&upload.bucket, &upload.name)
                    .with_generation(upload.generation)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        READ_ERROR.fetch_add(1, Ordering::SeqCst);
                        self.on_error(ex.clone(), &e);
                        tracing::error!("READ error {e:?}");
                        continue;
                    }
                };
                let mut transfer_size = 0;
                while let Some(result) = read.next().await {
                    match result {
                        Ok(b) => transfer_size += b.len(),
                        Err(e) => {
                            READ_ERROR.fetch_add(1, Ordering::SeqCst);
                            self.on_partial_error(ex.clone(), transfer_size, &e);
                            break;
                        }
                    }
                }
                if transfer_size != size {
                    continue;
                }
                let sample = Sample::success(ex);
                if let Err(_) = self.tx.send(sample) {
                    SEND_ERROR.fetch_add(1, Ordering::SeqCst);
                }
            }
            if let Err(error) = self
                .control
                .delete_object()
                .set_bucket(upload.bucket)
                .set_object(upload.name)
                .set_generation(upload.generation)
                .with_idempotency(true)
                .with_backoff_policy(backoff_policy::default())
                .with_retry_policy(RecommendedPolicy.with_time_limit(Duration::from_secs(30)))
                .with_attempt_timeout(Duration::from_secs(5))
                .send()
                .await
            {
                tracing::error!("DELETE error = {error:?}");
                DELETE_ERROR.fetch_add(1, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    async fn on_write(&self, ex: Experiment<'_>, upload: ResultObject) -> ResultObject {
        let upload = match upload {
            Ok(u) => u,
            Err(e) => {
                WRITE_ERROR.fetch_add(1, Ordering::SeqCst);
                self.on_error(ex, &e);
                return Err(e);
            }
        };
        let sample = Sample::success(ex);
        if let Err(_) = self.tx.send(sample) {
            SEND_ERROR.fetch_add(1, Ordering::SeqCst);
        }
        Ok(upload)
    }

    fn on_error(&self, ex: Experiment, error: &google_cloud_storage::Error) {
        WRITE_ERROR.fetch_add(1, Ordering::SeqCst);
        let sample = Sample::error(ex, &error);
        if let Err(_) = self.tx.send(sample) {
            SEND_ERROR.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn on_partial_error(
        &self,
        ex: Experiment,
        transfer_size: usize,
        error: &google_cloud_storage::Error,
    ) {
        WRITE_ERROR.fetch_add(1, Ordering::SeqCst);
        let sample = Sample::interrupted(ex, transfer_size, &error);
        if let Err(_) = self.tx.send(sample) {
            SEND_ERROR.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn random_object_name() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[derive(Clone, Debug)]
struct Experiment<'a> {
    task: i32,
    relative: Duration,
    iteration: u64,
    start: Instant,
    op: Operation,
    target_size: usize,
    name: &'a str,
}

#[derive(Clone, Debug)]
struct Sample {
    task: i32,
    iteration: u64,
    op_start: Duration,
    op: Operation,
    size: usize,
    transfer_size: usize,
    elapsed: Duration,
    object: String,
    result: Result,
    details: String,
}

impl Sample {
    const HEADER: &str = concat!(
        "Task,Iteration,IterationStart,Operation",
        ",Size,TransferSize,ElapsedMicroseconds,Object",
        ",Result,Details"
    );

    fn error(ex: Experiment, error: &google_cloud_storage::Error) -> Self {
        tracing::error!("experiment = {ex:?} error = {error:?}");
        Self {
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size: 0,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: Result::Error,
            details: format!(
                "W={};R={};S={};D={};{}",
                WRITE_ERROR.load(Ordering::SeqCst),
                READ_ERROR.load(Ordering::SeqCst),
                SEND_ERROR.load(Ordering::SeqCst),
                DELETE_ERROR.load(Ordering::SeqCst),
                format!("{error:?}").replace(",", ";")
            ),
        }
    }

    fn interrupted(
        ex: Experiment,
        transfer_size: usize,
        error: &google_cloud_storage::Error,
    ) -> Self {
        tracing::error!("experiment = {ex:?} error = {error:?}");
        Self {
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: Result::Interrupted,
            details: format!(
                "W={};R={};S={};D={};{}",
                WRITE_ERROR.load(Ordering::SeqCst),
                READ_ERROR.load(Ordering::SeqCst),
                SEND_ERROR.load(Ordering::SeqCst),
                DELETE_ERROR.load(Ordering::SeqCst),
                error.http_status_code().unwrap_or_default()
            ),
        }
    }

    fn success(ex: Experiment) -> Self {
        Self {
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size: ex.target_size,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: Result::Success,
            details: String::new(),
        }
    }

    fn to_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.task,
            self.iteration,
            self.op_start.as_micros(),
            self.op.name(),
            self.size,
            self.transfer_size,
            self.elapsed.as_micros(),
            self.object,
            self.result.name(),
            self.details,
        )
    }
}

#[derive(Clone, Debug)]
enum Operation {
    Resumable,
    SingleShot,
    Read0,
    Read1,
    Read2,
}

impl Operation {
    fn name(&self) -> &str {
        match self {
            Self::Resumable => "RESUMABLE",
            Self::SingleShot => "SINGLE_SHOT",
            Self::Read0 => "READ[0]",
            Self::Read1 => "READ[1]",
            Self::Read2 => "READ[2]",
        }
    }
}

#[derive(Clone, Debug)]
enum Result {
    Success,
    Error,
    Interrupted,
}

impl Result {
    fn name(&self) -> &str {
        match self {
            Self::Success => "OK",
            Self::Error => "ERR",
            Self::Interrupted => "INT",
        }
    }
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

const MIB: u64 = 1024 * 1024;

#[derive(Clone, Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    bucket_name: String,

    #[arg(long, default_value_t = 0, value_parser = parse_size_arg)]
    min_object_size: u64,
    #[arg(long, default_value_t = 4 * MIB, value_parser = parse_size_arg)]
    max_object_size: u64,

    #[arg(long, default_value_t = 1)]
    task_count: i32,

    #[arg(long, default_value_t = 1)]
    min_sample_count: u64,
}

fn parse_size_arg(arg: &str) -> anyhow::Result<u64> {
    let value = parse_size::parse_size(arg)?;
    Ok(value)
}
