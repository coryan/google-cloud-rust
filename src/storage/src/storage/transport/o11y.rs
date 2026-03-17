// Copyright 2026 Google LLC
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

use crate::storage::info::INSTRUMENTATION;
pub use gaxi::observability::DurationMetric;

pub fn duration_metric() -> DurationMetric {
    DurationMetric::new(&INSTRUMENTATION)
}

macro_rules! storage_client_request {
    ($client_method:literal) => {{
        use crate::storage::info::INSTRUMENTATION;
        use ::gaxi::observability::RequestRecorder;
        use ::gaxi::observability::attributes::keys::*;
        use ::gaxi::observability::attributes::otel_status_codes;
        use ::gaxi::observability::attributes::{
            GCP_CLIENT_LANGUAGE_RUST, GCP_CLIENT_REPO_GOOGLEAPIS, OTEL_KIND_INTERNAL,
            RPC_SYSTEM_HTTP,
        };
        use ::tracing::field::Empty;

        let recorder = RequestRecorder::new(INSTRUMENTATION.clone());
        let span = tracing::info_span!(
            "client_request",
            { OTEL_NAME } = concat!(
                env!("CARGO_CRATE_NAME"),
                "::client::Storage::",
                $client_method
            ),
            { OTEL_KIND } = OTEL_KIND_INTERNAL,
            { RPC_SYSTEM } = RPC_SYSTEM_HTTP, // Default to HTTP, can be overridden
            { RPC_SERVICE } = INSTRUMENTATION.service_name,
            { RPC_METHOD } = Empty,
            { GCP_CLIENT_SERVICE } = INSTRUMENTATION.service_name,
            { GCP_CLIENT_VERSION } = INSTRUMENTATION.client_version,
            { GCP_CLIENT_REPO } = GCP_CLIENT_REPO_GOOGLEAPIS,
            { GCP_CLIENT_ARTIFACT } = INSTRUMENTATION.client_artifact,
            { GCP_CLIENT_LANGUAGE } = GCP_CLIENT_LANGUAGE_RUST,
            // Fields to be recorded later
            { OTEL_STATUS_CODE } = otel_status_codes::UNSET,
            { OTEL_STATUS_DESCRIPTION } = Empty,
            { ERROR_TYPE } = Empty,
            { SERVER_ADDRESS } = Empty,
            { SERVER_PORT } = Empty,
            { URL_FULL } = Empty,
            { HTTP_REQUEST_METHOD } = Empty,
            { HTTP_RESPONSE_STATUS_CODE } = Empty,
            { HTTP_REQUEST_RESEND_COUNT } = Empty,
        );

        (span, recorder)
    }};
}

pub(crate) use storage_client_request;
