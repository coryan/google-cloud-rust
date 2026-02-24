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

//! This module contains types to export OpenTelemetry metrics to Google Cloud Monitoring.
//!
//! # Example
//! ```
//! use integration_tests_o11y::otlp::metrics::Builder;
//! use opentelemetry_sdk::metrics::SdkMeterProvider;
//! use opentelemetry::{global, KeyValue};
//! # async fn example() -> anyhow::Result<()> {
//! let provider: SdkMeterProvider = Builder::new("my-project", "my-service")
//!     .build()
//!     .await?;
//! // Make the provider available to the libraries and application.
//! global::set_meter_provider(provider.clone());
//! // Use the provider.
//! let meter = opentelemetry::global::meter("my-component");
//! let counter = meter.u64_counter("my_counter").build();
//! counter.add(1, &[KeyValue::new("my.key", "my.value")]);
//! # Ok(()) }
//! ```

use super::Error;
use super::{OTEL_KEY_GCP_PROJECT_ID, OTEL_KEY_SERVICE_NAME};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use google_cloud_api::model::MonitoredResource;
use google_cloud_logging_type::model::LogSeverity;
use google_cloud_logging_v2::client::LoggingServiceV2;
use google_cloud_logging_v2::model::{LogEntry, log_entry::Payload};
use google_cloud_wkt::Struct as WktStruct;
use opentelemetry::InstrumentationScope;
use opentelemetry::logs::{AnyValue, Severity as OtelSeverity};
use opentelemetry_sdk::Resource as SdkResource;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::logs::{LogBatch, LogExporter, SdkLogRecord, SdkLoggerProvider};
use opentelemetry_sdk::resource::ResourceDetector;
use serde_json::json;
use std::sync::Mutex;

/// Creates a "Cloud Logging"-based implementation of `SdkLoggerProvider` .
///
/// This builder creates a `SdkLogerProvider` configured to export metrics via
/// the Google Cloud Logging API (`logging.googleapis.com`). It automatically
/// handles authentication by injecting OAuth2 tokens into every request.
///
/// The resulting provider is configured with:
/// - **Transport:** HTTP via the `google-cloud-logging-v2` client library.
/// - **Endpoint:** `https://logging.googleapis.com`.
/// - **Resource Attributes:** sets `gcp.project_id` and `service.name` as
///   required by Cloud Logging.
///
/// # Example
/// ```
/// use integration_tests_o11y::otlp::logs::Builder;
/// # async fn example() -> anyhow::Result<()> {
/// let provider = Builder::new("my-project", "my-service")
///     .build()
///     .await?;
/// # Ok(()) }
/// ```
pub struct Builder {
    project_id: String,
    service_name: String,
    detector: Option<Box<dyn ResourceDetector>>,
    client: Option<LoggingServiceV2>,
}

impl Builder {
    /// Creates a new builder with the required Google Cloud project ID and service name.
    ///
    /// # Parameters
    /// * `project_id` - The Google Cloud project ID. This is attached as the `gcp.project_id`
    ///   resource attribute, which is required by Cloud Trace.
    /// * `service_name` - The logical name of the service. Attached as `service.name` resource
    ///   attribute, used by Cloud Trace to group and identify services.
    pub fn new<P, S>(project_id: P, service_name: S) -> Self
    where
        P: Into<String>,
        S: Into<String>,
    {
        Self {
            project_id: project_id.into(),
            service_name: service_name.into(),
            detector: None,
            client: None,
        }
    }

    /// Sets the client used for export.
    ///
    /// This can be useful when the client needs custom credentials, endpoints,
    /// or any other configuration.
    pub fn with_client(mut self, client: LoggingServiceV2) -> Self {
        self.client = Some(client);
        self
    }

    /// Sets the resource detector.
    pub fn with_detector<D>(mut self, detector: D) -> Self
    where
        D: ResourceDetector + 'static,
    {
        self.detector = Some(Box::new(detector));
        self
    }

    /// Builds and initializes the `SdkTracerProvider`.
    pub async fn build(self) -> Result<SdkLoggerProvider, Error> {
        let log_name = format!(
            "projects/{}/logs/{}%2Frequest-errors",
            self.project_id, self.service_name
        );
        let resource = opentelemetry_sdk::Resource::builder()
            .with_attributes(vec![
                opentelemetry::KeyValue::new(OTEL_KEY_GCP_PROJECT_ID, self.project_id),
                opentelemetry::KeyValue::new(OTEL_KEY_SERVICE_NAME, self.service_name),
            ])
            .build();

        let exporter = Exporter::new(self.client, log_name)
            .await
            .map_err(Error::create_exporter)?;
        let provider = SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        Ok(provider)
    }
}

#[derive(Debug)]
struct Exporter {
    client: LoggingServiceV2,
    log_name: String,
    handle: tokio::runtime::Handle,
    resource: Mutex<Option<MonitoredResource>>,
}

impl Exporter {
    pub async fn new(client: Option<LoggingServiceV2>, log_name: String) -> anyhow::Result<Self> {
        if let Some(client) = client {
            return Self::from_client(client, log_name);
        }
        let client = LoggingServiceV2::builder().build().await?;
        Self::from_client(client, log_name)
    }

    fn from_client(client: LoggingServiceV2, log_name: String) -> anyhow::Result<Self> {
        let handle = tokio::runtime::Handle::try_current()?;
        Ok(Self {
            client,
            log_name,
            handle,
            resource: Default::default(),
        })
    }

    fn map_record(record: &SdkLogRecord, _scope: &InstrumentationScope) -> LogEntry {
        let timestamp = record
            .timestamp()
            .and_then(|t| google_cloud_wkt::Timestamp::try_from(t).ok());
        let payload = record.body().map(Self::map_payload);
        let span_id = record
            .trace_context()
            .map(|c| c.span_id.to_string())
            .unwrap_or_default();
        let trace_id = record
            .trace_context()
            .map(|c| c.trace_id.to_string())
            .unwrap_or_default();

        let mut entry = LogEntry::new()
            .set_or_clear_timestamp(timestamp)
            .set_span_id(span_id)
            .set_trace(trace_id)
            .set_payload(payload)
            .set_labels(
                record
                    .attributes_iter()
                    .map(|(key, value)| (key.to_string(), Self::map_attribute_value(value))),
            );
        if let Some(s) = record.severity_number() {
            entry = entry.set_severity(Self::map_severity(s));
        }
        entry
    }

    fn map_attribute_value(value: &AnyValue) -> String {
        match value {
            AnyValue::String(v) => v.to_string(),
            AnyValue::Boolean(v) => v.to_string(),
            AnyValue::Double(v) => v.to_string(),
            AnyValue::Int(v) => v.to_string(),
            AnyValue::Bytes(v) => format!("[{} bytes]", v.len()),
            AnyValue::ListAny(v) => format!("[a list with {} elements]", v.len()),
            AnyValue::Map(v) => format!("[a map with {} elements]", v.len()),
            _ => "an unknown value type".to_string(),
        }
    }

    fn map_severity(severity: OtelSeverity) -> LogSeverity {
        match severity {
            OtelSeverity::Debug
            | OtelSeverity::Debug2
            | OtelSeverity::Debug3
            | OtelSeverity::Debug4 => LogSeverity::Debug,
            OtelSeverity::Error
            | OtelSeverity::Error2
            | OtelSeverity::Error3
            | OtelSeverity::Error4 => LogSeverity::Error,
            OtelSeverity::Fatal
            | OtelSeverity::Fatal2
            | OtelSeverity::Fatal3
            | OtelSeverity::Fatal4 => LogSeverity::Emergency,
            OtelSeverity::Info
            | OtelSeverity::Info2
            | OtelSeverity::Info3
            | OtelSeverity::Info4 => LogSeverity::Info,
            OtelSeverity::Trace
            | OtelSeverity::Trace2
            | OtelSeverity::Trace3
            | OtelSeverity::Trace4 => LogSeverity::Debug,
            OtelSeverity::Warn
            | OtelSeverity::Warn2
            | OtelSeverity::Warn3
            | OtelSeverity::Warn4 => LogSeverity::Warning,
        }
    }

    fn map_payload(body: &AnyValue) -> Payload {
        match body {
            AnyValue::Int(v) => Payload::JsonPayload(Box::new(WktStruct::from_iter([(
                "intValue".to_string(),
                json!(v),
            )]))),
            AnyValue::Double(v) => Payload::JsonPayload(Box::new(WktStruct::from_iter([(
                "doubleValue".to_string(),
                json!(v),
            )]))),
            AnyValue::String(v) => Payload::TextPayload(v.to_string()),
            AnyValue::Boolean(v) => Payload::JsonPayload(Box::new(WktStruct::from_iter([(
                "boolValue".to_string(),
                json!(v),
            )]))),
            AnyValue::Bytes(v) => Payload::JsonPayload(Box::new(WktStruct::from_iter([(
                "bytesValue".to_string(),
                json!(URL_SAFE_NO_PAD.encode(v.as_slice())),
            )]))),
            AnyValue::Map(v) => Payload::JsonPayload(
                WktStruct::from_iter(
                    v.iter()
                        .map(|(key, value)| (key.to_string(), Self::to_json(value))),
                )
                .into(),
            ),
            AnyValue::ListAny(v) => Payload::JsonPayload(
                WktStruct::from_iter([(
                    "listValue".to_string(),
                    serde_json::Value::Array(v.iter().map(Self::to_json).collect()),
                )])
                .into(),
            ),
            unknown => Payload::TextPayload(format!("unknown payload type: {unknown:?}")),
        }
    }

    fn to_json(value: &AnyValue) -> serde_json::Value {
        match value {
            AnyValue::Int(v) => json!(v),
            AnyValue::Double(v) => json!(v),
            AnyValue::String(v) => serde_json::Value::String(v.to_string()),
            AnyValue::Boolean(v) => json!(v),
            AnyValue::Bytes(v) => json!(URL_SAFE_NO_PAD.encode(v.as_slice())),
            AnyValue::Map(v) => serde_json::Value::Object(serde_json::Map::from_iter(
                v.iter()
                    .map(|(key, value)| (key.to_string(), Self::to_json(value))),
            )),
            AnyValue::ListAny(v) => serde_json::Value::Array(v.iter().map(Self::to_json).collect()),
            unknown => json!(format!("unknown payload type: {unknown:?}")),
        }
    }
}

impl LogExporter for Exporter {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        let request = self
            .client
            .write_log_entries()
            .set_log_name(&self.log_name)
            .set_entries(
                batch
                    .iter()
                    .map(|(record, scope)| Self::map_record(record, scope)),
            );
        let request = self
            .resource
            .lock()
            .expect("never poisoned")
            .iter()
            .fold(request, |req, res| req.set_resource(res.clone()));

        // The thread collecting a batch is not part of the tokio runtime. We need to spawn the job
        // in a thread that is. The result is Result<Result<_>> because the spawn adds its own error type.
        let _ = self
            .handle
            .spawn(request.send())
            .await
            .map_err(|e| {
                OTelSdkError::InternalFailure(format!("error exporting to Cloud Logging: {e:?}"))
            })?
            .map_err(|e| {
                OTelSdkError::InternalFailure(format!("error exporting to Cloud Logging: {e:?}"))
            })?;
        Ok(())
    }

    fn set_resource(&mut self, resource: &SdkResource) {
        // Convert to a resource in the Cloud Logging format and store for later
        // use.
        let resource = super::monitored_resource::map_resource(resource);
        let _ = self
            .resource
            .lock()
            .expect("never poisoned")
            .insert(resource);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Anonymous;
    use crate::mock_collector::MockCollector;
    use crate::otlp::trace::Builder as TracerProviderBuilder;
    use google_cloud_gax::Result as GaxResult;
    use google_cloud_gax::options::RequestOptions;
    use google_cloud_gax::response::Response as GaxResponse;
    use google_cloud_logging_v2::model::{WriteLogEntriesRequest, WriteLogEntriesResponse};
    use google_cloud_logging_v2::stub::LoggingServiceV2 as LoggingServiceV2Stub;
    use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider};
    use opentelemetry::trace::{TraceContextExt, TracerProvider};
    use std::sync::Arc;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    const WANT: [(&str, &str); 6] = [
        ("gcp.client.service", "storage"),
        ("gcp.client.version", "1.2.3"),
        ("gcp.client.repo", "googleapis/google-cloud-rust"),
        ("gcp.client.artifact", "google-cloud-storage"),
        ("rpc.system.name", "GRPC"),
        (
            "gcp.method",
            "google.cloud.storage.v2.Storage/delete_bucket",
        ),
    ];

    #[tokio::test]
    async fn export_with_mock_client() -> anyhow::Result<()> {
        let (client, captured) = test_client().await?;
        let provider = Builder::new("test-project", "test-service")
            .with_client(client)
            .build()
            .await
            .expect("logger provider builds successfully");

        const NAME: &str = "experimental-gcp.client.request.error";
        let mut want = WANT.clone();
        want.sort_by(|a, b| a.0.cmp(b.0));
        let want = want;
        let logger = provider.logger("test-logger");
        let mut record = logger.create_log_record();
        record.set_event_name(NAME);
        record.set_body(AnyValue::from("a test log outside a span"));
        record.add_attributes(want);
        record.set_severity_number(opentelemetry::logs::Severity::Info);
        logger.emit(record);

        provider.force_flush()?;

        let requests = captured
            .lock()
            .expect("never poisoned")
            .drain(..)
            .collect::<Vec<_>>();

        let request = match &requests[..] {
            [got] => got,
            _ => panic!("expected one request, got={requests:?}"),
        };
        let resource = request
            .resource
            .as_ref()
            .expect("the resource metrics should have a resource: {rm:?}");
        let got = resource
            .labels
            .iter()
            .find(|(key, val)| *key == OTEL_KEY_GCP_PROJECT_ID && *val == "test-project");
        assert!(got.is_some(), "{got:?}\n{resource:?}");
        let got = resource
            .labels
            .iter()
            .find(|(key, val)| *key == OTEL_KEY_SERVICE_NAME && *val == "test-service");
        assert!(got.is_some(), "{got:?}\n{resource:?}");

        let untraced = match &request.entries[..] {
            [a] => a,
            _ => panic!("expected one log entries, got={:?}", request.entries),
        };
        assert!(untraced.trace.is_empty(), "{untraced:?}");

        Ok(())
    }

    #[tokio::test]
    async fn export_with_mock_client_and_traces() -> anyhow::Result<()> {
        // We need to initialize a trace provider to verify logs work with traces.
        let mock_collector = MockCollector::default();
        let endpoint = mock_collector.start().await;
        let trace_provider = TracerProviderBuilder::new("test-project", "test-service")
            .with_credentials(Anonymous::new().build())
            .with_endpoint(endpoint)
            .build()
            .await
            .expect("failed to build provider");
        let _tracer = trace_provider.tracer("test-tracer");

        let (client, captured) = test_client().await?;
        let provider = Builder::new("test-project", "test-service")
            .with_client(client)
            .build()
            .await
            .expect("logger provider builds successfully");
        let logger = provider.logger("test-logger");

        const NAME: &str = "experimental-gcp.client.request.error";
        let mut want = WANT.clone();
        want.sort_by(|a, b| a.0.cmp(b.0));
        let want = want;
        let span = tracing::info_span!("test-span");
        {
            let enter = span.entered();
            // assert!(!enter.is_disabled(), "{enter:?}");
            let trace_id = enter.context().span().span_context().trace_id();
            let span_id = enter.context().span().span_context().span_id();

            let mut record = logger.create_log_record();
            record.set_event_name(NAME);
            record.set_body(AnyValue::from("test log inside a span"));
            record.add_attributes(want);
            record.set_severity_number(opentelemetry::logs::Severity::Info);
            record.set_trace_context(trace_id, span_id, None);
            logger.emit(record);
        }
        provider.force_flush()?;

        let requests = captured
            .lock()
            .expect("never poisoned")
            .drain(..)
            .collect::<Vec<_>>();

        let request = match &requests[..] {
            [got] => got,
            _ => panic!("expected one request, got={requests:?}"),
        };

        let untraced = match &request.entries[..] {
            [a] => a,
            _ => panic!("expected one log entries, got={:?}", request.entries),
        };
        assert!(untraced.trace.is_empty(), "{untraced:?}");
        Ok(())
    }

    /// Creates a new test client that collects requests to a vector.
    async fn test_client()
    -> anyhow::Result<(LoggingServiceV2, Arc<Mutex<Vec<WriteLogEntriesRequest>>>)> {
        let capture = Arc::new(Mutex::new(Vec::<WriteLogEntriesRequest>::new()));
        let mut mock_client = MockLoggingService::new();
        let capturing = capture.clone();
        mock_client
            .expect_write_log_entries()
            .times(1..)
            .returning(move |req, _| {
                capturing.lock().expect("never poisoned").push(req);
                Ok(GaxResponse::from(WriteLogEntriesResponse::new()))
            });

        let client = LoggingServiceV2::from_stub(mock_client);
        Ok((client, capture))
    }

    mockall::mock! {
        #[derive(Debug)]
        LoggingService {}

        impl LoggingServiceV2Stub for LoggingService {
            async fn write_log_entries(
                &self,
                request: WriteLogEntriesRequest,
                options: RequestOptions,
            ) -> GaxResult<GaxResponse<WriteLogEntriesResponse>>;
        }
    }
}
