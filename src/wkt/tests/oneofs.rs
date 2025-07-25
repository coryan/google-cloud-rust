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
mod tests {
    use common::MessageWithOneOf;
    use common::message_with_one_of::{Message, Mixed, SingleString, TwoStrings};
    use google_cloud_wkt::Duration;
    use serde_json::{Value, json};
    use test_case::test_case;
    type Result = anyhow::Result<()>;

    #[test_case(MessageWithOneOf::new(), json!({}))]
    #[test_case(MessageWithOneOf::new().set_string_contents(""), json!({"stringContents": ""}))]
    #[test_case(MessageWithOneOf::new().set_string_contents("abc"), json!({"stringContents": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_string_contents_one("abc"), json!({"stringContentsOne": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_string_contents_two("abc"), json!({"stringContentsTwo": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_message_value(Message::new()), json!({"messageValue": {}}))]
    #[test_case(MessageWithOneOf::new().set_another_message(Message::new()), json!({"anotherMessage": {}}))]
    #[test_case(MessageWithOneOf::new().set_string(""), json!({"string": ""}))]
    #[test_case(MessageWithOneOf::new().set_string("abc"), json!({"string": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_duration(Duration::clamp(1, 500_000_000)), json!({"duration": "1.5s"}))]
    fn test_ser(input: MessageWithOneOf, want: Value) -> Result {
        let got = serde_json::to_value(input)?;
        assert_eq!(got, want);
        Ok(())
    }

    #[test_case(MessageWithOneOf::new(), json!({}))]
    #[test_case(MessageWithOneOf::new().set_string_contents(""), json!({"stringContents": ""}))]
    #[test_case(MessageWithOneOf::new().set_string_contents("abc"), json!({"stringContents": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_string_contents_one("abc"), json!({"stringContentsOne": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_string_contents_two("abc"), json!({"stringContentsTwo": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_message_value(Message::new()), json!({"messageValue": {}}))]
    #[test_case(MessageWithOneOf::new().set_another_message(Message::new()), json!({"anotherMessage": {}}))]
    #[test_case(MessageWithOneOf::new().set_string(""), json!({"string": ""}))]
    #[test_case(MessageWithOneOf::new().set_string("abc"), json!({"string": "abc"}))]
    #[test_case(MessageWithOneOf::new().set_duration(Duration::clamp(1, 500_000_000)), json!({"duration": "1.5s"}))]
    fn test_de(want: MessageWithOneOf, input: Value) -> Result {
        let got = serde_json::from_value::<MessageWithOneOf>(input)?;
        assert_eq!(got, want);
        Ok(())
    }

    #[test_case(r#"{"string_contents":      null}"#, MessageWithOneOf::new().set_string_contents(""))]
    #[test_case(r#"{"string_contents_one":  null}"#, MessageWithOneOf::new().set_string_contents_one(""))]
    #[test_case(r#"{"string_contents_two":  null}"#, MessageWithOneOf::new().set_string_contents_two(""))]
    #[test_case(r#"{"message_value":        null}"#, MessageWithOneOf::new().set_message_value(Message::default()))]
    #[test_case(r#"{"another_message":      null}"#, MessageWithOneOf::new().set_another_message(Message::default()))]
    #[test_case(r#"{"string":               null}"#, MessageWithOneOf::new().set_string(""))]
    #[test_case(r#"{"duration":             null}"#, MessageWithOneOf::new().set_duration(Duration::default()))]
    fn test_null_is_default(input: &str, want: MessageWithOneOf) -> Result {
        let got = serde_json::from_str::<MessageWithOneOf>(input)?;
        assert_eq!(got, want);
        Ok(())
    }

    #[test_case(json!({"stringContentsOne": "abc", "stringContentsTwo": "cde"}))]
    #[test_case(json!({"stringContentsTwo": "abc", "stringContentsOne": "cde"}))]
    #[test_case(json!({"anotherMessage": {}, "string": "cde"}))]
    #[test_case(json!({"anotherMessage": {}, "duration": "1.5s"}))]
    #[test_case(json!({"string": "", "duration": "1.5s"}))]
    fn test_dup_field_errors(input: Value) -> Result {
        let got = serde_json::from_value::<MessageWithOneOf>(input).unwrap_err();
        assert!(got.is_data(), "{got:?}");
        Ok(())
    }

    #[test_case(json!({"unknown": "test-value"}))]
    #[test_case(json!({"unknown": "test-value", "moreUnknown": {"a": 1, "b": 2}}))]
    fn test_unknown(input: Value) -> Result {
        let deser = serde_json::from_value::<MessageWithOneOf>(input.clone())?;
        let got = serde_json::to_value(deser)?;
        assert_eq!(got, input);
        Ok(())
    }

    #[test]
    fn test_oneof_single_string() -> Result {
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
    fn test_oneof_two_strings() -> Result {
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
    fn test_oneof_one_message() -> Result {
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
    fn test_oneof_mixed() -> Result {
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
