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

//! Helper functions to convert from the well-known types to and from their
//! Prost versions.

impl std::convert::From<crate::Duration> for prost_types::Duration {
    fn from(value: crate::Duration) -> Self {
        Self {
            seconds: value.seconds(),
            nanos: value.nanos(),
        }
    }
}

impl std::convert::From<prost_types::Duration> for crate::Duration {
    fn from(value: prost_types::Duration) -> Self {
        Self::clamp(value.seconds, value.nanos)
    }
}

impl std::convert::From<crate::Timestamp> for prost_types::Timestamp {
    fn from(value: crate::Timestamp) -> Self {
        Self {
            seconds: value.seconds(),
            nanos: value.nanos(),
        }
    }
}

impl std::convert::From<prost_types::Timestamp> for crate::Timestamp {
    fn from(value: prost_types::Timestamp) -> Self {
        Self::clamp(value.seconds, value.nanos)
    }
}

impl std::convert::From<crate::NullValue> for i32 {
    fn from(_value: crate::NullValue) -> Self {
        0
    }
}

impl std::convert::From<i32> for crate::NullValue {
    fn from(_value: i32) -> Self {
        Self
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn from_prost_duration() {
        let input = prost_types::Duration {
            seconds: 123,
            nanos: 456,
        };
        let got = crate::Duration::from(input);
        assert_eq!(got, crate::Duration::clamp(123, 456));
    }

    #[test]
    fn from_wkt_duration() {
        let input = crate::Duration::clamp(123, 456);
        let got = prost_types::Duration::from(input);
        assert_eq!(
            got,
            prost_types::Duration {
                seconds: 123,
                nanos: 456
            }
        );
    }

    #[test]
    fn from_prost_timestamp() {
        let input = prost_types::Timestamp {
            seconds: 123,
            nanos: 456,
        };
        let got = crate::Timestamp::from(input);
        assert_eq!(got, crate::Timestamp::clamp(123, 456));
    }

    #[test]
    fn from_wkt_timestamp() {
        let input = crate::Timestamp::clamp(123, 456);
        let got = prost_types::Timestamp::from(input);
        assert_eq!(
            got,
            prost_types::Timestamp {
                seconds: 123,
                nanos: 456
            }
        );
    }
}
