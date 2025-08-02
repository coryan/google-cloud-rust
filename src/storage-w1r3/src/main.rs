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
use google_cloud_storage::client::Storage;
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
    let args = Args::parse();
    println!("# Args = {args:?}");

    let client = Storage::builder().build().await?;

    let (tx, mut rx) = tokio::sync::broadcast::channel(128);

    let tasks = (0..args.task_count)
        .map(|i| tokio::spawn(runner(client.clone(), args.clone(), i, tx.clone())))
        .collect::<Vec<_>>();
    drop(tx);

    println!("{}", Sample::HEADER);
    loop {
        match rx.recv().await {
            Ok(sample) => println!("{}", sample.to_row()),
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
    ]
    .into_iter()
    .for_each(|(key, value)| {
        println!("# {key} = {value}");
    });
    println!("# EOF");
    Ok(())
}

async fn runner(client: Storage, args: Args, id: i32, tx: Sender<Sample>) -> anyhow::Result<()> {
    let control = google_cloud_storage::client::StorageControl::builder()
        .build()
        .await?;
    let buffer = bytes::Bytes::from_owner(
        rand::rng()
            .sample_iter(Uniform::new_inclusive(u8::MIN, u8::MAX)?)
            .take(args.max_object_size as usize)
            .collect::<Vec<_>>(),
    );
    let size = Uniform::new_inclusive(args.min_object_size, args.max_object_size)?;

    for iteration in 0..args.min_sample_count {
        let size = rand::rng().sample(size) as usize;
        let name = random_object_name();

        let write_start = Instant::now();
        let upload = client
            .upload_object(
                format!("projects/_/buckets/{}", &args.bucket_name),
                &name,
                buffer.slice(0..size),
            )
            .send_unbuffered()
            .await;
        let upload = match upload {
            Ok(u) => u,
            Err(_) => {
                WRITE_ERROR.fetch_add(1, Ordering::SeqCst);
                continue;
            }
        };
        let sample = Sample {
            id,
            iteration,
            size,
            transfer_size: size,
            op: Operation::Write,
            elapsed: Instant::now() - write_start,
        };
        match tx.send(sample) {
            Ok(_) => {}
            Err(_) => {
                SEND_ERROR.fetch_add(1, Ordering::SeqCst);
            }
        };
        for op in [Operation::Read0, Operation::Read1, Operation::Read2] {
            let read_start = Instant::now();
            let mut read = match client
                .read_object(&upload.bucket, &upload.name)
                .with_generation(upload.generation)
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    READ_ERROR.fetch_add(1, Ordering::SeqCst);
                    continue;
                }
            };
            let mut transfer_size = 0;
            while let Some(b) = read.next().await {
                if let Ok(b) = b {
                    transfer_size += b.len();
                } else {
                }
            }
            let sample = Sample {
                id,
                iteration,
                size,
                transfer_size,
                op,
                elapsed: Instant::now() - read_start,
            };
            match tx.send(sample) {
                Ok(_) => {}
                Err(_) => {
                    SEND_ERROR.fetch_add(1, Ordering::SeqCst);
                }
            };
        }
        if let Err(_) = control
            .delete_object()
            .set_bucket(upload.bucket)
            .set_object(upload.name)
            .set_generation(upload.generation)
            .send()
            .await
        {
            DELETE_ERROR.fetch_add(1, Ordering::SeqCst);
        }
    }
    Ok(())
}

fn random_object_name() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[derive(Clone, Debug)]
struct Sample {
    id: i32,
    iteration: u64,
    op: Operation,
    size: usize,
    transfer_size: usize,
    elapsed: Duration,
}

impl Sample {
    const HEADER: &str = "Task,Iteration,Operation,Size,TransferSize,ElapsedMicroseconds";

    fn to_row(&self) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.id,
            self.iteration,
            self.op.name(),
            self.size,
            self.transfer_size,
            self.elapsed.as_micros()
        )
    }
}

#[derive(Clone, Debug)]
enum Operation {
    Write,
    Read0,
    Read1,
    Read2,
}

impl Operation {
    fn name(&self) -> &str {
        match self {
            Self::Write => "WRITE",
            Self::Read0 => "READ[0]",
            Self::Read1 => "READ[1]",
            Self::Read2 => "READ[2]",
        }
    }
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
