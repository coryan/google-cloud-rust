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

macro_rules! impl_primitive {
    ($t: ty) => {
        impl Convert<$t> for $t {
            fn cnv(self) -> $t {
                self
            }
        }
    };
}

impl_primitive!(bool);
impl_primitive!(f32);
impl_primitive!(i32);
impl_primitive!(u32);
impl_primitive!(f64);
impl_primitive!(i64);
impl_primitive!(u64);
impl_primitive!(String);
impl_primitive!(bytes::Bytes);

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

impl Convert<crate::FieldMask> for prost_types::FieldMask {
    fn cnv(self) -> crate::FieldMask {
        crate::FieldMask::default().set_paths(self.paths)
    }
}

impl Convert<prost_types::FieldMask> for crate::FieldMask {
    fn cnv(self) -> prost_types::FieldMask {
        prost_types::FieldMask { paths: self.paths }
    }
}

impl Convert<crate::Timestamp> for prost_types::Timestamp {
    fn cnv(self) -> crate::Timestamp {
        crate::Timestamp::clamp(self.seconds, self.nanos)
    }
}

impl Convert<prost_types::Timestamp> for crate::Timestamp {
    fn cnv(self) -> prost_types::Timestamp {
        prost_types::Timestamp {
            seconds: self.seconds(),
            nanos: self.nanos(),
        }
    }
}

impl Convert<crate::Struct> for prost_types::Struct {
    fn cnv(self) -> crate::Struct {
        self.fields
            .into_iter()
            .map(|(k, v)| (k.cnv(), v.cnv()))
            .collect()
    }
}

impl Convert<prost_types::Struct> for crate::Struct {
    fn cnv(self) -> prost_types::Struct {
        prost_types::Struct {
            fields: self.into_iter().map(|(k, v)| (k.cnv(), v.cnv())).collect(),
        }
    }
}

impl Convert<crate::Value> for prost_types::Value {
    fn cnv(self) -> crate::Value {
        match self.kind {
            None => crate::Value::Null,
            Some(kind) => match kind {
                prost_types::value::Kind::NullValue(_) => crate::Value::Null,
                prost_types::value::Kind::NumberValue(v) => {
                    let number =
                        serde_json::Number::from_f64(v).expect("JSON numbers cannot be NaN");
                    serde_json::Value::Number(number)
                }
                prost_types::value::Kind::StringValue(v) => crate::Value::String(v),
                prost_types::value::Kind::BoolValue(v) => crate::Value::Bool(v),
                prost_types::value::Kind::StructValue(v) => crate::Value::Object(v.cnv()),
                prost_types::value::Kind::ListValue(v) => crate::Value::Array(v.cnv()),
            },
        }
    }
}

impl Convert<prost_types::Value> for crate::Value {
    fn cnv(self) -> prost_types::Value {
        prost_types::Value { kind: None }
    }
}

impl Convert<crate::ListValue> for prost_types::ListValue {
    fn cnv(self) -> crate::ListValue {
        self.values.into_iter().map(|v| v.cnv()).collect()
    }
}

impl Convert<prost_types::ListValue> for crate::ListValue {
    fn cnv(self) -> prost_types::ListValue {
        prost_types::ListValue {
            values: self.into_iter().map(|v| v.cnv()).collect(),
        }
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
    fn primitive_bool() {
        let input: bool = true;
        let got: bool = input.cnv();
        assert_eq!(got, input);
    }

    #[test_case::test_case(0 as f32)]
    #[test_case::test_case(0 as i32)]
    #[test_case::test_case(0 as u32)]
    #[test_case::test_case(0 as f64)]
    #[test_case::test_case(0 as i64)]
    #[test_case::test_case(0 as u64)]
    fn primitive_numeric<T>(input: T)
    where
        T: std::fmt::Debug + Copy + PartialEq + Convert<T>,
    {
        let got: T = input.cnv();
        assert_eq!(got, input);
    }

    #[test]
    fn primitive_string() {
        let input = "abc".to_string();
        let got: String = input.cnv();
        assert_eq!(&got, "abc");
    }

    #[test]
    fn primitive_bytes() {
        let input = bytes::Bytes::from_static(b"abc");
        let got: bytes::Bytes = input.clone().cnv();
        assert_eq!(got, input);
    }

    #[test]
    fn from_prost_duration() {
        let input = prost_types::Duration {
            seconds: 123,
            nanos: 456,
        };
        let got: crate::Duration = input.cnv();
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
    fn from_prost_field_mask() {
        let input = prost_types::FieldMask {
            paths: ["a", "b", "c"].map(str::to_string).to_vec(),
        };
        let got: crate::FieldMask = input.cnv();
        assert_eq!(got, crate::FieldMask::default().set_paths(["a", "b", "c"].map(str::to_string).to_vec()));
    }

    #[test]
    fn from_wkt_field_mask() {
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
        let got: crate::Timestamp = input.cnv();
        assert_eq!(got, crate::Timestamp::clamp(123, 456));
    }

    #[test]
    fn from_wkt_timestamp() {
        let input = crate::Timestamp::clamp(123, 456);
        let got: prost_types::Timestamp = input.cnv();
        assert_eq!(
            got,
            prost_types::Timestamp {
                seconds: 123,
                nanos: 456
            }
        );
    }
}
