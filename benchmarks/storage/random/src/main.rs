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

use clap::Parser;

const DESCRIPTION: &str = concat!(
    "This benchmark repeatedly reads ranges from a set of Cloud Storage objects.",
    " In each iteration of the benchmark the number of concurrent ranges,",
    " the size of the ranges, and the location of the ranges is selected at random.",
    " The API used for the download is also selected at random.",
    " The benchmark runs multiple tasks concurrently, all running identical loops."
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = args::Args::parse();
    args.validate()?;
    Ok(())
}
