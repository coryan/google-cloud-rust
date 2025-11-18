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

use crate::sample::Protocol;

use super::experiment::{Experiment, Range};
use super::sample::{Attempt, Sample};
use anyhow::Result;
use google_cloud_storage::client::Storage;
use google_cloud_storage::model_ext::ReadRange;
use std::time::Instant;

pub async fn iteration(
    task: usize,
    iteration: u64,
    client: &Storage,
    test_start: Instant,
    experiment: Experiment,
) -> Vec<Sample> {
    let start = Instant::now();
    let relative_start = start - test_start;

    let running = experiment
        .ranges
        .iter()
        .map(|r| attempt(client, r))
        .collect::<Vec<_>>();
    let elapsed = Instant::now() - start;

    futures::future::join_all(running)
        .await
        .into_iter()
        .zip(experiment.ranges)
        .enumerate()
        .map(|(i, (result, range))| {
            let (ttfb, ttlb, details) = match result {
                Ok(a) => (a.ttfb, a.ttlb, "OK"),
                Err(e) => {
                    tracing::error!("error on range {i}: {e:?}");
                    (elapsed, elapsed, "ERROR")
                }
            };
            Sample {
                protocol: Protocol::Json,
                ttfb,
                ttlb,
                details: details.to_string(),
                task,
                iteration,
                range_id: i,
                start: relative_start,
                range_length: range.read_length,
                object: range.object_name,
            }
        })
        .collect()
}

async fn attempt(client: &Storage, range: &Range) -> Result<Attempt> {
    let start = Instant::now();
    let mut reader = client
        .read_object(range.bucket_name.clone(), range.object_name.clone())
        .set_read_range(ReadRange::segment(range.read_offset, range.read_offset))
        .send()
        .await?;
    let ttfb = Instant::now() - start;
    while reader.next().await.transpose()?.is_some() {}
    let ttlb = Instant::now() - start;
    Ok(Attempt { ttfb, ttlb })
}
