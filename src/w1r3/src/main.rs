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
use google_cloud_gax::paginator::ItemPaginator;
use google_cloud_gax::retry_policy::RetryPolicyExt;
use google_cloud_storage::backoff_policy;
use google_cloud_storage::client::{Storage, StorageControl};
use google_cloud_storage::model::Object;
use google_cloud_storage::retry_policy::RecommendedPolicy;
use rand::{
    Rng,
    distr::{Alphanumeric, Uniform},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::Sender;

static WRITE_ERROR: AtomicU64 = AtomicU64::new(0);
static READ_ERROR: AtomicU64 = AtomicU64::new(0);
static DELETE_ERROR: AtomicU64 = AtomicU64::new(0);
static SEND_ERROR: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = enable_tracing();

    let args = Args::parse();
    if args.min_object_size > args.max_object_size {
        return Err(anyhow::Error::msg("invalid object size range"));
    }
    tracing::info!("{args:?}");

    let client = Storage::builder().build().await?;

    let (sample_tx, mut sample_rx) = tokio::sync::mpsc::channel::<Sample>(1024 * args.task_count);
    let test_start = Instant::now();
    let buffer = bytes::Bytes::from_owner(
        rand::rng()
            .sample_iter(Uniform::new_inclusive(u8::MIN, u8::MAX)?)
            .take(args.max_object_size as usize)
            .collect::<Vec<_>>(),
    );
    let run = uuid::Uuid::new_v4().to_string();
    let tasks = (0..args.task_count)
        .map(|id| {
            let task = Task {
                run: run.clone(),
                id,
                start: test_start,
                client: client.clone(),
                tx: sample_tx.clone(),
                buffer: buffer.clone(),
            };
            tokio::spawn(task.run(args.clone()))
        })
        .collect::<Vec<_>>();
    drop(sample_tx);

    println!("{}", Sample::HEADER);
    let mut sample_count = 0_u64;
    while let Some(sample) = sample_rx.recv().await {
        sample_count += 1;
        println!("{}", sample.to_row());
    }

    tracing::info!("cleaning up");
    let control = StorageControl::builder().build().await?;
    let (delete_tx, mut delete_rx) = tokio::sync::mpsc::channel::<Object>(1024 * args.task_count);
    let delete_batch_size = 1024_usize;
    let delete_control = control.clone();
    let deleter = tokio::spawn(async move {
        let _guard = enable_tracing();
        let mut batch = Vec::new();
        while let Some(object) = delete_rx.recv().await {
            batch.push(delete(&delete_control, object));
            if batch.len() >= delete_batch_size {
                join_deletes(
                    batch.split_off(0)
                )
                .await;
            }
        }
        join_deletes(batch).await;
    });
    let mut list = control
        .list_objects()
        .set_parent(format!("projects/_/buckets/{}", args.bucket_name))
        .set_prefix(run.clone())
        .by_item();
    while let Some(o) = list.next().await {
        let Ok(object) = o else {
            break;
        };
        let _ = delete_tx.send(object).await;
    }
    drop(delete_tx);

    if let Err(e) = deleter.await {
        tracing::error!("cannot join deleter {e}");
    }

    for t in tasks {
        if let Err(e) = t.await {
            tracing::error!("cannot join task {e}");
        }
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

fn delete(
    control: &StorageControl,
    object: Object,
) -> impl Future<Output = Result<(), google_cloud_storage::Error>> {
    control
        .delete_object()
        .set_bucket(object.bucket)
        .set_object(object.name)
        .set_generation(object.generation)
        .with_idempotency(true)
        .with_backoff_policy(backoff_policy::default())
        .with_retry_policy(RecommendedPolicy.with_time_limit(Duration::from_secs(60)))
        .send()
}

async fn join_deletes<F>(batch: Vec<F>)
where
    F: Future<Output = Result<(), google_cloud_storage::Error>>,
{
    let done = futures::future::join_all(batch).await;
    for j in done {
        if let Err(e) = j {
            tracing::info!("DELETE ERROR = {e:?}");
            DELETE_ERROR.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[derive(Clone)]
struct Task {
    run: String,
    client: Storage,
    start: Instant,
    buffer: bytes::Bytes,
    id: usize,
    tx: Sender<Sample>,
}

type ResultObject = google_cloud_storage::Result<google_cloud_storage::model::Object>;

impl Task {
    async fn run(self, args: Args) {
        let _guard = enable_tracing();
        if self.id % 128 == 0 {
            tracing::info!("Task::run({})", self.id);
        }
        let size = Uniform::new_inclusive(args.min_object_size, args.max_object_size).unwrap();

        for iteration in 0..args.min_sample_count {
            let name = random_object_name(&self.run);
            let size = rand::rng().sample(size) as usize;
            let (write_op, threshold) = if rand::rng().random_bool(0.5) {
                (Operation::Resumable, 0_usize)
            } else {
                (Operation::SingleShot, size)
            };

            let write_start = Instant::now();
            let ex = Experiment {
                run: self.run.clone(),
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
                    run: self.run.clone(),
                    task: self.id,
                    relative: read_start - self.start,
                    iteration,
                    start: read_start,
                    op,
                    target_size: size,
                    name: &upload.name,
                };
                let sample = match self.download(&upload).await {
                    (_, Ok(_)) => Sample::success(ex),
                    (0, Err(e)) => Sample::error(ex, &e),
                    (partial, Err(e)) => Sample::interrupted(ex, partial, &e),
                };
                if let Err(_) = self.tx.send(sample).await {
                    SEND_ERROR.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }

    async fn download(
        &self,
        object: &google_cloud_storage::model::Object,
    ) -> (usize, Result<(), google_cloud_storage::Error>) {
        let mut read = match self
            .client
            .read_object(&object.bucket, &object.name)
            .with_generation(object.generation)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return (0, Err(e)),
        };
        let mut transfer_size = 0;
        while let Some(result) = read.next().await {
            match result {
                Ok(b) => transfer_size += b.len(),
                Err(e) => return (transfer_size, Err(e)),
            }
        }
        (transfer_size, Ok(()))
    }

    async fn on_write(&self, ex: Experiment<'_>, upload: ResultObject) -> ResultObject {
        let upload = match upload {
            Ok(u) => {
                if let Err(_) = self.tx.send(Sample::success(ex)).await {
                    SEND_ERROR.fetch_add(1, Ordering::SeqCst);
                }
                u
            }
            Err(e) => {
                if let Err(_) = self.tx.send(Sample::error(ex, &e)).await {
                    SEND_ERROR.fetch_add(1, Ordering::SeqCst);
                }
                WRITE_ERROR.fetch_add(1, Ordering::SeqCst);
                return Err(e);
            }
        };
        Ok(upload)
    }
}

fn random_object_name(prefix: &str) -> String {
    let suffix: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    format!("{prefix}/{suffix}")
}

#[derive(Clone, Debug)]
struct Experiment<'a> {
    run: String,
    task: usize,
    relative: Duration,
    iteration: u64,
    start: Instant,
    op: Operation,
    target_size: usize,
    name: &'a str,
}

#[derive(Clone, Debug)]
struct Sample {
    run: String,
    task: usize,
    iteration: u64,
    op_start: Duration,
    op: Operation,
    size: usize,
    transfer_size: usize,
    elapsed: Duration,
    object: String,
    result: ExperimentResult,
    details: String,
}

impl Sample {
    const HEADER: &str = concat!(
        "Experiment,Task,Iteration,IterationStart,Operation",
        ",Size,TransferSize,ElapsedMicroseconds,Object",
        ",Result,Details"
    );

    fn error(ex: Experiment, error: &google_cloud_storage::Error) -> Self {
        tracing::error!("experiment = {ex:?} error = {error:?}");
        Self {
            run: ex.run,
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size: 0,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: ExperimentResult::Error,
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
        tracing::error!("experiment = {ex:?} download interrupted");
        Self {
            run: ex.run,
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: ExperimentResult::Interrupted,
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

    fn success(ex: Experiment) -> Self {
        Self {
            run: ex.run,
            task: ex.task,
            iteration: ex.iteration,
            op_start: ex.relative,
            size: ex.target_size,
            transfer_size: ex.target_size,
            op: ex.op,
            elapsed: Instant::now() - ex.start,
            object: ex.name.to_string(),
            result: ExperimentResult::Success,
            details: String::new(),
        }
    }

    fn to_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            self.run,
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
enum ExperimentResult {
    Success,
    Error,
    Interrupted,
}

impl ExperimentResult {
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
    task_count: usize,

    #[arg(long, default_value_t = 1)]
    min_sample_count: u64,
}

fn parse_size_arg(arg: &str) -> anyhow::Result<u64> {
    let value = parse_size::parse_size(arg)?;
    Ok(value)
}
