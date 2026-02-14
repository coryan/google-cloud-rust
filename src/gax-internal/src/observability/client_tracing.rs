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

use crate::observability::attributes::keys::{OTEL_STATUS_CODE, OTEL_STATUS_DESCRIPTION};
use crate::observability::attributes::otel_status_codes;
use crate::observability::errors::ErrorType;
use crate::options::InstrumentationClientInfo;
use google_cloud_gax::error::Error;
use google_cloud_gax::response::Response;
use opentelemetry_semantic_conventions::trace as otel_trace;
use tracing::Span;

#[macro_export]
macro_rules! client_request_v {
    ($span_name:expr, $method_name:expr, $info:expr) => {{
        use $crate::observability::attributes::keys::*;
        use $crate::observability::attributes::{
            GCP_CLIENT_LANGUAGE_RUST, GCP_CLIENT_REPO_GOOGLEAPIS, RPC_SYSTEM_HTTP,
            otel_status_codes,
        };
        tracing::info_span!(
            "client_request",
            "gax.client.span" = true, // Marker field
            { OTEL_NAME } = $span_name,
            { OTEL_KIND } = $crate::observability::attributes::OTEL_KIND_INTERNAL,
            { RPC_SYSTEM } = RPC_SYSTEM_HTTP, // Default to HTTP, can be overridden
            { RPC_SERVICE } = $info.service_name,
            { RPC_METHOD } = $method_name,
            { GCP_CLIENT_SERVICE } = $info.service_name,
            { GCP_CLIENT_VERSION } = $info.client_version,
            { GCP_CLIENT_REPO } = GCP_CLIENT_REPO_GOOGLEAPIS,
            { GCP_CLIENT_ARTIFACT } = $info.client_artifact,
            { GCP_CLIENT_LANGUAGE } = GCP_CLIENT_LANGUAGE_RUST,
            // Fields to be recorded later
            { OTEL_STATUS_CODE } = otel_status_codes::UNSET,
            { OTEL_STATUS_DESCRIPTION } = ::tracing::field::Empty,
            { ERROR_TYPE } = ::tracing::field::Empty,
            { SERVER_ADDRESS } = ::tracing::field::Empty,
            { SERVER_PORT } = ::tracing::field::Empty,
            { URL_FULL } = ::tracing::field::Empty,
            { HTTP_REQUEST_METHOD } = ::tracing::field::Empty,
            { HTTP_RESPONSE_STATUS_CODE } = ::tracing::field::Empty,
            { HTTP_REQUEST_RESEND_COUNT } = ::tracing::field::Empty,
        )
    }};
}

#[macro_export]
macro_rules! client_request_span {
    ($client:expr, $method:expr, $info:expr) => {
        $crate::client_request_v!(
            concat!(env!("CARGO_PKG_NAME"), "::", $client, "::", $method),
            $method,
            $info
        )
    };
}

/// Creates a new tracing span for a client request.
///
/// This span represents the logical request operation and is used to track
/// the overall duration and status of the request, including retries.
///
/// # Example
///
/// ```compile_fail
/// let span = create_client_request_span(
///     "google_cloud_storage::client::Client::upload_chunk",
///     "upload_chunk",
///     &INSTRUMENTATION_CLIENT_INFO
/// );
/// ```
pub fn create_client_request_span(
    span_name: &str,
    method_name: &str,
    instrumentation: &'static InstrumentationClientInfo,
) -> Span {
    client_request_v!(span_name, method_name, instrumentation)
}

/// Records the final status on the client request span.
pub fn record_client_request_span<T>(result: &Result<Response<T>, Error>, span: &Span) {
    match result {
        Ok(_) => {
            span.record(OTEL_STATUS_CODE, otel_status_codes::OK);
        }
        Err(err) => {
            span.record(OTEL_STATUS_CODE, otel_status_codes::ERROR);
            let error_type = ErrorType::from_gax_error(err);
            span.record(otel_trace::ERROR_TYPE, error_type.as_str());
            span.record(OTEL_STATUS_DESCRIPTION, err.to_string());
        }
    }
}

pub trait ResultExt {
    fn record_in_span(self, span: &Span) -> Self;
}

impl<T> ResultExt for std::result::Result<T, Error> {
    fn record_in_span(self, span: &Span) -> Self {
        match &self {
            Ok(_) => {
                span.record(OTEL_STATUS_CODE, otel_status_codes::OK);
            }
            Err(err) => {
                span.record(OTEL_STATUS_CODE, otel_status_codes::ERROR);
                let error_type = ErrorType::from_gax_error(err);
                span.record(otel_trace::ERROR_TYPE, error_type.as_str());
                span.record(OTEL_STATUS_DESCRIPTION, err.to_string());
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::InstrumentationClientInfo;
    use google_cloud_test_utils::test_layer::{AttributeValue, TestLayer};
    use std::collections::HashMap;

    const INFO: InstrumentationClientInfo = InstrumentationClientInfo {
        service_name: "test.service",
        client_version: "1.2.3",
        client_artifact: "google-cloud-test",
        default_host: "example.com",
    };

    #[tokio::test]
    async fn test_create_client_request_span() {
        let guard = TestLayer::initialize();
        let _span = create_client_request_span(
            "google_cloud_test::service::TestMethod",
            "TestMethod",
            &INFO,
        );

        let expected_attributes: HashMap<String, AttributeValue> = [
            (OTEL_NAME, "google_cloud_test::service::TestMethod".into()),
            (OTEL_KIND, "Internal".into()),
            (otel_trace::RPC_SYSTEM, "http".into()),
            (otel_trace::RPC_SERVICE, "test.service".into()),
            (otel_trace::RPC_METHOD, "TestMethod".into()),
            (GCP_CLIENT_SERVICE, "test.service".into()),
            (GCP_CLIENT_VERSION, "1.2.3".into()),
            (GCP_CLIENT_REPO, "googleapis/google-cloud-rust".into()),
            (GCP_CLIENT_ARTIFACT, "google-cloud-test".into()),
            (GCP_CLIENT_LANGUAGE, "rust".into()),
            (OTEL_STATUS_CODE, "UNSET".into()),
            ("gax.client.span", true.into()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        let captured = TestLayer::capture(&guard);
        assert_eq!(captured.len(), 1, "captured spans: {:?}", captured);
        let attributes = &captured[0].attributes;
        assert_eq!(*attributes, expected_attributes);
    }

    #[tokio::test]
    async fn test_record_client_request_span_ok() {
        let guard = TestLayer::initialize();
        let span = create_client_request_span(
            "google_cloud_test::service::TestMethod",
            "TestMethod",
            &INFO,
        );
        let _enter = span.enter();

        let response = google_cloud_gax::response::Response::from(());
        record_client_request_span(&Ok(response), &span);

        let captured = TestLayer::capture(&guard);
        assert_eq!(captured.len(), 1, "captured spans: {:?}", captured);
        let attributes = &captured[0].attributes;
        assert_eq!(attributes.get(OTEL_STATUS_CODE), Some(&"OK".into()));
    }

    #[tokio::test]
    async fn test_record_client_request_span_err() {
        let guard = TestLayer::initialize();
        let span = create_client_request_span(
            "google_cloud_test::service::TestMethod",
            "TestMethod",
            &INFO,
        );
        let _enter = span.enter();

        let error = google_cloud_gax::error::Error::timeout("test timeout");
        record_client_request_span::<()>(&Err(error), &span);

        let captured = TestLayer::capture(&guard);
        assert_eq!(captured.len(), 1, "captured spans: {:?}", captured);
        let attributes = &captured[0].attributes;
        assert_eq!(attributes.get(OTEL_STATUS_CODE), Some(&"ERROR".into()));
        assert_eq!(
            attributes.get(otel_trace::ERROR_TYPE),
            Some(&"CLIENT_TIMEOUT".into())
        );
        assert_eq!(
            attributes.get(OTEL_STATUS_DESCRIPTION),
            Some(&"the request exceeded the request deadline test timeout".into())
        );
    }
}
