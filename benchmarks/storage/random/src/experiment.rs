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

use super::args::Args;
use super::sample::Protocol;
use anyhow::Result;
use rand::distr::Uniform;
use rand::seq::IndexedRandom;

pub struct Range {
    pub bucket_name: String,
    pub object_name: String,
    pub read_offset: u64,
    pub read_length: u64,
}

pub struct Experiment {
    pub ranges: Vec<Range>,
    pub protocol: Protocol,
}

pub struct ExperimentGenerator {
    read_count: Uniform<u64>,
    read_offset: Uniform<u64>,
    read_length: Uniform<u64>,
    objects: Vec<String>,
    bucket_name: String,
    protocols: Vec<Protocol>,
}

impl ExperimentGenerator {
    pub fn new(args: &Args, objects: Vec<String>) -> Result<Self> {
        let read_count =
            Uniform::new_inclusive(args.min_concurrent_reads, args.max_concurrent_reads)?;
        let max_offset = args.min_range_count * args.max_range_size;
        let read_offset = Uniform::new_inclusive(0, max_offset)?;
        let read_length = Uniform::new_inclusive(args.min_range_size, args.max_range_size)?;
        Ok(Self {
            read_count,
            read_offset,
            read_length,
            objects,
            bucket_name: format!("projects/_/buckets/{}", args.bucket_name),
            protocols: args.protocols.clone(),
        })
    }

    pub fn generate(&self) -> Experiment {
        use rand::Rng;
        let mut rng = rand::rng();
        let read_count = rng.sample(self.read_count);
        let read_length = rng.sample(self.read_length);
        let protocol = self
            .protocols
            .choose(&mut rng)
            .expect("protocols selection is not empty")
            .to_owned();

        let ranges = (0..read_count)
            .map(move |_| {
                let read_offset = rng.sample(self.read_offset);
                let object_name = self
                    .objects
                    .choose(&mut rng)
                    .expect("object list is not empty")
                    .clone();
                Range {
                    read_offset,
                    read_length,
                    object_name,
                    bucket_name: self.bucket_name.clone(),
                }
            })
            .collect::<Vec<_>>();

        Experiment { ranges, protocol }
    }
}
