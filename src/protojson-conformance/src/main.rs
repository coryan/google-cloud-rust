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

use prost::{
    Message,
    bytes::{Buf, BufMut},
};
use protojson_conformance::conformance::conformance_response::Result;
use protojson_conformance::conformance::{ConformanceRequest, ConformanceResponse};
use std::io::{Read, Write, stdin};

fn main() -> std::io::Result<()> {
    let mut buf = vec![0; 4];
    loop {
        buf.resize(4, 0);
        if stdin().read_exact(buf.as_mut_slice()).is_err() {
            // Treat EOF (or other errors) reading the header as the end of testing.
            return Ok(());
        }
        let size = buf.as_slice().get_u32_le() as usize;

        buf.resize(size, 0);
        stdin().read_exact(buf.as_mut_slice())?;

        let request = ConformanceRequest::decode(buf.as_slice())?;
        let response = handle(request)?;
        let len = response.encoded_len();
        buf.clear();
        buf.put_u32_le(len as u32);
        response.encode(&mut buf)?;
        assert_eq!(len + 4, buf.len());

        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&buf)?;
        stdout.flush()?;
    }
}

fn handle(request: ConformanceRequest) -> std::io::Result<ConformanceResponse> {
    use protojson_conformance::conformance::WireFormat;
    use protojson_conformance::conformance::conformance_response;
    let result = match request.requested_output_format() {
        WireFormat::Unspecified => {
            conformance_response::Result::ParseError("output format was not specified".to_string())
        }
        WireFormat::Json => handle_json(request),
        WireFormat::Jspb => {
            conformance_response::Result::Skipped("Jspb output is not supported".to_string())
        }
        WireFormat::TextFormat => {
            conformance_response::Result::Skipped("TEXTFORMAT output is not supported".to_string())
        }
        WireFormat::Protobuf => {
            conformance_response::Result::Skipped("PROTOBUF output is not supported".to_string())
        }
    };

    Ok(ConformanceResponse {
        result: Some(result),
        ..Default::default()
    })
}

fn handle_json(request: ConformanceRequest) -> Result {
    use protojson_conformance::conformance::conformance_request;
    match request.payload {
        None => Result::ParseError("no payload".to_string()),
        Some(conformance_request::Payload::JsonPayload(p)) => {
            handle_json_message(&request.message_type, p)
        }
        Some(conformance_request::Payload::JspbPayload(_)) => {
            Result::Skipped("JSPB input is not supported".to_string())
        }
        Some(conformance_request::Payload::TextPayload(_)) => {
            Result::Skipped("TEXT input is not supported".to_string())
        }
        Some(conformance_request::Payload::ProtobufPayload(_)) => {
            Result::Skipped("PROTOBUF input is not supported".to_string())
        }
    }
}

fn handle_json_message(message_type: &str, payload: String) -> Result {
    match message_type {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => {
            Result::Skipped("Proto2 messages skipped".to_string())
        }
        "protobuf_test_messages.proto3.TestAllTypesProto3" => roundtrip(payload),
        _ => Result::ParseError(format!("unknown message type {message_type}")),
    }
}

fn roundtrip(payload: String) -> Result {
    use protojson_conformance::generated::test_protos::TestAllTypesProto3;

    match serde_json::from_str::<TestAllTypesProto3>(&payload) {
        Ok(input) => serde_json::to_string(&input)
            .map(Result::JsonPayload)
            .unwrap_or_else(|e| Result::SerializeError(e.to_string())),
        Err(e) => Result::ParseError(format!(
            "error parsing JSON input for TestAllTypesProto3: {e:?}"
        )),
    }
}
