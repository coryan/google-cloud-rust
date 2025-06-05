// Copyright 2024 Google LLC
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

//! Test serialization for oneofs.
//!
//! This shows (1) what we want the generator to produce for `oneof` fields,
//! and (2) that this serializes as we want.

#[cfg(test)]
mod test {
    use google_cloud_wkt::Duration;
    use serde_json::json;
    type TestResult = anyhow::Result<()>;

    #[allow(dead_code)]
    mod protos {
        use google_cloud_wkt as wkt;
        include!("generated/mod.rs");

        impl<'de> serde::de::Deserialize<'de> for MessageWithOneOf {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Visitor;
                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = MessageWithOneOf;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct MessageWithOneOf")
                    }
                    fn visit_seq<V>(self, mut seq: V) -> Result<MessageWithOneOf, V::Error>
                    where
                        V: serde::de::SeqAccess<'de>,
                    {
                        use serde::de::Error;
                        let result = MessageWithOneOf::new();
                        let result = seq
                            .next_element::<Option<message_with_one_of::SingleString>>()?
                            .ok_or_else(|| V::Error::invalid_length(0, &self))?
                            .into_iter()
                            .fold(result, |r, v| r.set_single_string(v));
                        let result = seq
                            .next_element::<Option<message_with_one_of::TwoStrings>>()?
                            .ok_or_else(|| V::Error::invalid_length(1, &self))?
                            .into_iter()
                            .fold(result, |r, v| r.set_two_strings(v));
                        Ok(result)
                    }
                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        use serde::de::Error;
                        let mut result = MessageWithOneOf::new();
                        while let Some(key) = map.next_key::<String>()? {
                            match key.as_str() {
                                "stringContents" => {
                                    if result.single_string.is_some() {
                                        return Err(A::Error::duplicate_field("single_string"));
                                    }
                                    let value = map.next_value::<String>()?;
                                    result.single_string = Some(
                                        message_with_one_of::SingleString::StringContents(value),
                                    );
                                }
                                "stringContentsOne" => {
                                    if result.two_strings.is_some() {
                                        return Err(A::Error::duplicate_field("two_strings"));
                                    }
                                    let value = map.next_value::<String>()?;
                                    result.two_strings = Some(
                                        message_with_one_of::TwoStrings::StringContentsOne(value),
                                    );
                                }
                                "stringContentsTwo" => {
                                    if result.two_strings.is_some() {
                                        return Err(A::Error::duplicate_field("two_strings"));
                                    }
                                    let value = map.next_value::<String>()?;
                                    result.two_strings = Some(
                                        message_with_one_of::TwoStrings::StringContentsTwo(value),
                                    );
                                }
                                unknown => {
                                    let value = map.next_value::<serde_json::Value>()?;
                                    result._unknown_fields.insert(unknown.to_string(), value);
                                }
                            }
                        }
                        Ok(result)
                    }
                }
                deserializer.deserialize_any(Visitor)
            }
        }
    }
    use protos::MessageWithOneOf;
    use protos::message_with_one_of::{Message, Mixed, SingleString, TwoStrings};

    #[test]
    fn test_oneof_overlapping_variants_v2() -> TestResult {
        let input = json!({"stringContentsOne": "overlap1"});
        let got = serde_json::from_value::<MessageWithOneOf>(input)?;
        assert!(
            matches!(&got.two_strings, Some(TwoStrings::StringContentsOne(s)) if s == "overlap1"),
            "{got:?}"
        );
        let input = json!({"stringContentsTwo": "overlap2"});
        let got = serde_json::from_value::<MessageWithOneOf>(input)?;
        assert!(
            matches!(&got.two_strings, Some(TwoStrings::StringContentsTwo(s)) if s == "overlap2"),
            "{got:?}"
        );

        let input = json!({"stringContentsOne": "overlap1", "stringContentsTwo": "overlap2"});
        let got = serde_json::from_value::<MessageWithOneOf>(input);
        assert!(got.is_err(), "{got:?}");
        Ok(())
    }

    #[test]
    fn test_oneof_single_string() -> TestResult {
        let input = MessageWithOneOf::default()
            .set_single_string(SingleString::StringContents("test-only".to_string()));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "stringContents": "test-only"
        });
        assert_eq!(got, want);
        Ok(())
    }

    #[test]
    fn test_oneof_two_strings() -> TestResult {
        let input = MessageWithOneOf::default()
            .set_two_strings(TwoStrings::StringContentsTwo("test-only".to_string()));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "stringContentsTwo": "test-only"
        });
        assert_eq!(got, want);
        Ok(())
    }

    #[test]
    fn test_oneof_one_message() -> TestResult {
        let input = MessageWithOneOf::default()
            .set_message_value(Message::default().set_parent("parent-value"));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "messageValue": { "parent": "parent-value" }
        });
        assert_eq!(got, want);
        Ok(())
    }

    #[test]
    fn test_oneof_mixed() -> TestResult {
        let input = MessageWithOneOf::default()
            .set_another_message(Message::default().set_parent("parent-value"));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "anotherMessage": { "parent": "parent-value" }
        });
        assert_eq!(got, want);

        let input =
            MessageWithOneOf::default().set_mixed(Mixed::String("string-value".to_string()));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "string": "string-value"
        });
        assert_eq!(got, want);

        let input = MessageWithOneOf::default().set_duration(Duration::clamp(123, 456_000_000));
        let got = serde_json::to_value(&input)?;
        let want = json!({
            "duration": "123.456s"
        });
        assert_eq!(got, want);

        Ok(())
    }
}
