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

use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Sample {
    task: usize,
    iteration: u64,
    start: Duration,
    range_id: i64,
    range_size: usize,
    protocol: Protocol,
    ttfb: Duration,
    ttlb: Duration,
    object: String,
    details: String,
}

impl Sample {
    pub const HEADER: &str = concat!(
        "Task,Iteration,IterationStart,RangeId",
        ",RangeSize,Protocol,TtfbMicroseconds,TtlbMicroseconds",
        ",Object,Details"
    );

    pub fn to_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.task,
            self.iteration,
            self.start.as_micros(),
            self.range_id,
            self.range_size,
            self.protocol.name(),
            self.ttfb.as_micros(),
            self.ttlb.as_micros(),
            self.object,
            self.details,
        )
    }
}

#[derive(Clone, Debug)]
pub enum Protocol {
    Bidi,
    Json,
}

impl Protocol {
    pub fn name(&self) -> &str {
        match self {
            Self::Bidi => "bidi",
            Self::Json => "json",
        }
    }
}
