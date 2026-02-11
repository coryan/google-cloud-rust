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

use super::Anonymous;
use crate::mock_collector::MockCollector;
use crate::otlp::CloudTelemetryTracerProviderBuilder;
use google_cloud_storage::client::Storage;
use google_cloud_test_utils::test_layer::{TestLayer, TestLayerGuard};
use httptest::{Expectation, Server, matchers::*, responders::status_code};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Validates that HTTP tracing makes it all the way to OTLP collectors like
/// Cloud Telemetry.
///
/// This test sets up an in-memory service endpoint and OTLP collector
/// endpoint and installs a standard Rust tracing -> authenticated OTLP tracer
/// provider.  Then it uses the showcase client library to make a request and
/// checks that the right spans are collected.
///
/// This makes sure that the end-to-end system of tracing to OpenTelemetry
/// works as intended, value types are preserved, etc.
pub async fn to_otlp() -> anyhow::Result<()> {
    // 1. Start Mock OTLP Collector
    let mock_collector = MockCollector::default();
    let otlp_endpoint = mock_collector.start().await;

    // 2. Configure OTel Provider
    let provider = CloudTelemetryTracerProviderBuilder::new("test-project", "integration-tests")
        .with_endpoint(otlp_endpoint)
        .with_credentials(Anonymous::new().build())
        .build()
        .await?;

    // 3. Install Tracing Subscriber
    let _guard = tracing_subscriber::Registry::default()
        .with(crate::tracing::layer(provider.clone()))
        .set_default();

    // 4. Start Mock HTTP Server and configure the client.
    let (_layer_guard, server, client) = setup_fake_storage().await;
    const CONTENTS: &str = "the quick brown fox jumps over the lazy dog";
    server.expect(
        Expectation::matching(all_of![
            request::method_path("GET", "/storage/v1/b/test-bucket/o/test-object"),
            request::query(url_decoded(contains(("alt", "media")))),
        ])
        .respond_with(
            status_code(200)
                .body(CONTENTS)
                .append_header(
                    "x-goog-hash",
                    "crc32c=PBj01g==,md5=d63R1fQSI9VYL8pzalyzNQ==",
                )
                .append_header("x-goog-generation", 123456789)
                .append_header("x-goog-metageneration", 234)
                .append_header("x-goog-stored-content-length", CONTENTS.len())
                .append_header("x-goog-stored-content-encoding", "identity")
                .append_header("x-goog-storage-class", "STANDARD")
                .append_header("content-language", "en")
                .append_header("content-type", "text/plain")
                .append_header("content-disposition", "inline")
                .append_header("etag", "etagval"),
        ),
    );

    // 6. Make Request
    let mut reader = client
        .read_object("projects/_/buckets/test-bucket", "test-object")
        .send()
        .await?;
    while let Some(_) = reader.next().await.transpose()? {}

    // 7. Flush Spans
    let _ = provider.force_flush();

    // 8. Verify Spans
    let requests = mock_collector.requests.lock().unwrap();
    assert!(
        !requests.is_empty(),
        "Should have received at least one OTLP request"
    );

    let request = &requests[0];
    assert!(
        !request.resource_spans.is_empty(),
        "Should have received at least one resource span"
    );
    let scope_spans = &request.resource_spans[0].scope_spans;
    assert!(
        !scope_spans.is_empty(),
        "request {request:?} should have scope spans"
    );
    let spans = &scope_spans[0].spans;
    assert!(!spans.is_empty(), "{request:?} should have spans");

    // Verify we captured the client span
    let client_span = spans.iter().find(|s| s.kind == 3 /* CLIENT */); // 3 is SPAN_KIND_CLIENT
    assert!(client_span.is_some(), "Should have a CLIENT span");

    // 9. Verify HTTP Span Details
    let client_span = client_span.unwrap();
    assert_eq!(
        client_span.name,
        "GET /storage/v1/b/test-bucket/o/test-object"
    );

    let attributes: std::collections::HashMap<String, _> = client_span
        .attributes
        .iter()
        .map(|kv| (kv.key.clone(), kv.value.clone().unwrap()))
        .collect();

    // Helper to get string value from AnyValue
    let get_string = |key: &str| -> Option<String> {
        attributes.get(key).and_then(|v| match &v.value {
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s)) => {
                Some(s.clone())
            }
            _ => None,
        })
    };

    assert_eq!(get_string("http.request.method").as_deref(), Some("GET"));
    assert_eq!(
        get_string("gcp.client.repo").as_deref(),
        Some("googleapis/google-cloud-rust")
    );
    assert!(get_string("gcp.client.version").is_some(), "{attributes:?}");

    Ok(())
}

async fn setup_fake_storage() -> (TestLayerGuard, Server, Storage) {
    let guard = TestLayer::initialize();
    let server = Server::run();
    let endpoint = server.url("/").to_string();
    let endpoint = endpoint.trim_end_matches('/');
    let client = Storage::builder()
        .with_endpoint(endpoint)
        .with_credentials(Anonymous::new().build())
        .build()
        .await
        .expect("failed to build client");

    (guard, server, client)
}
