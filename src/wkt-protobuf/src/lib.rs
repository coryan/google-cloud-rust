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

pub struct Duration(pub prost_types::Duration);

impl std::convert::From<wkt::Duration> for Duration {
    fn from(value: wkt::Duration) -> Self {
        Self(prost_types::Duration {
            seconds: value.seconds(),
            nanos: value.nanos(),
        })
    }
}

impl std::convert::From<Duration> for wkt::Duration {
    fn from(value: Duration) -> Self {
        Self::clamp(value.0.seconds, value.0.nanos)
    }
}

pub struct Timestamp(pub prost_types::Timestamp);

impl std::convert::From<wkt::Timestamp> for Timestamp {
    fn from(value: wkt::Timestamp) -> Self {
        Self(prost_types::Timestamp {
            seconds: value.seconds(),
            nanos: value.nanos(),
        })
    }
}

impl std::convert::From<Timestamp> for wkt::Timestamp {
    fn from(value: Timestamp) -> Self {
        Self::clamp(value.0.seconds, value.0.nanos)
    }
}
