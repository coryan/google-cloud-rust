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

/// Converts from `Self` into `T`.
pub trait Convert<T>: Sized {
    fn cnv(self) -> T;
}

impl<T, U> Convert<U> for T where T: std::convert::Into<U> {
    fn cnv(self) -> U {
        self.into()
    }
}

impl Convert<crate::Duration> for prost_types::Duration {
    fn cnv(self) -> crate::Duration {
        crate::Duration::clamp(self.seconds, self.nanos)
    }
}

impl Convert<prost_types::Duration> for crate::Duration {
    fn cnv(self) -> prost_types::Duration {
        prost_types::Duration {
            seconds: self.seconds(),
            nanos: self.nanos(),
        }
    }
}

impl Convert<crate::Timestamp> for prost_types::Timestamp {
    fn cnv(self) -> crate::Timestamp {
        crate::Timestamp::clamp(self.seconds, self.nanos)
    }
}

impl Convert<prost_types::Timestamp> for crate::Timestamp {
    fn cnv(self) -> prost_types::Timestamp {
        prost_types::Timestamp { seconds: self.seconds(), nanos: self.nanos() }
    }
}

impl Convert<crate::NullValue> for i32 {
    fn cnv(self) -> crate::NullValue {
        crate::NullValue
    }
}

impl Convert<i32> for crate::NullValue {
    fn cnv(self) -> i32 {
        prost_types::NullValue::NullValue as i32
    }
}

impl Convert<crate::NullValue> for prost_types::NullValue {
    fn cnv(self) -> crate::NullValue {
        crate::NullValue
    }
}

impl Convert<prost_types::NullValue> for crate::NullValue {
    fn cnv(self) -> prost_types::NullValue {
        prost_types::NullValue::NullValue
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_types() {
        let input : i64 = 0;
        let got : i64 = input.cnv();
        assert_eq!(got, input);

        let input : f64 = 0.0;
        let got : f64 = input.cnv();
        assert_eq!(got, input);

        let input : bool = true;
        let got : bool = input.cnv();
        assert_eq!(got, input);

        let input = "abc".to_string();
        let got : String = input.cnv();
        assert_eq!(&got, "abc");
    }

    #[test]
    fn from_prost_duration() {
        let input = prost_types::Duration {
            seconds: 123,
            nanos: 456,
        };
        let got  : crate::Duration = input.cnv();
        assert_eq!(got, crate::Duration::clamp(123, 456));
    }

    #[test]
    fn from_wkt_duration() {
        let input = crate::Duration::clamp(123, 456);
        let got: prost_types::Duration = input.cnv();
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
        let got : crate::Timestamp = input.cnv();
        assert_eq!(got, crate::Timestamp::clamp(123, 456));
    }

    #[test]
    fn from_wkt_timestamp() {
        let input = crate::Timestamp::clamp(123, 456);
        let got : prost_types::Timestamp = input.cnv();
        assert_eq!(
            got,
            prost_types::Timestamp {
                seconds: 123,
                nanos: 456
            }
        );
    }

}
