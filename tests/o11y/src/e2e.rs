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

pub mod logs;
pub mod metrics;
pub mod resource_detector;
pub mod showcase;
pub mod storage;

use super::otlp::logs::Builder as LoggerProviderBuilder;
use super::otlp::metrics::Builder as MeterProviderBuilder;
use super::otlp::trace::Builder as TracerProviderBuilder;
use google_cloud_auth::credentials::{Builder as CredentialsBuilder, Credentials};
use google_cloud_gax::error::rpc::Code;
use google_cloud_logging_v2::client::LoggingServiceV2;
use google_cloud_monitoring_v3::client::MetricService;
use google_cloud_monitoring_v3::model::{ListTimeSeriesResponse, TimeInterval};
use google_cloud_trace_v1::client::TraceService;
use google_cloud_trace_v1::model::Trace;
use google_cloud_wkt::Timestamp;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use resource_detector::TestResourceDetector;
use std::time::{Duration, SystemTime};
use tokio::sync::OnceCell;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;

pub const SERVICE_NAME: &str = "e2e-telemetry-test";
static PROVIDERS: OnceCell<anyhow::Result<Providers>> = OnceCell::const_new();

#[derive(Debug, Clone)]
pub struct Providers {
    pub trace: SdkTracerProvider,
    pub metrics: SdkMeterProvider,
    pub logs: SdkLoggerProvider,
}

/// Waits for a trace to appear in Cloud Trace.
///
/// Traces may take a few minutes to propagate from the collector endpoints to
/// the service. This function retrieves the trace, polling if the trace is
/// not found.
pub async fn wait_for_trace(project_id: &str, trace_id: &str) -> anyhow::Result<Trace> {
    let client = TraceService::builder().build().await?;

    // Because we are limited by quota, start with a backoff.
    // Traces can take several minutes to propagate after they have been written.
    // Implement a generous retry loop to account for this.
    let backoff_delays = [10, 60, 120, 120, 120];
    let mut trace = None;

    for delay in backoff_delays {
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

        match client
            .get_trace()
            .set_project_id(project_id)
            .set_trace_id(trace_id)
            .send()
            .await
        {
            Ok(t) => {
                trace = Some(t);
                break;
            }
            Err(e) => {
                if let Some(status) = e.status() {
                    if status.code == Code::NotFound || status.code == Code::Internal {
                        println!(
                            "Trace not found yet (or internal error), retrying... Error: {:?}",
                            e
                        );
                        continue;
                    }
                }
                return Err(e.into());
            }
        }
    }

    let trace = trace.ok_or_else(|| anyhow::anyhow!("Timed out waiting for trace"))?;
    Ok(trace)
}

pub async fn try_get_metric(
    client: &MetricService,
    project_id: &str,
    metric_name: &str,
    label: (&str, &str),
) -> anyhow::Result<Option<ListTimeSeriesResponse>> {
    let end = Timestamp::try_from(SystemTime::now())?;
    let start = Timestamp::try_from(SystemTime::now() - Duration::from_secs(300))?;
    let (key, value) = label;
    let response = client
        .list_time_series()
        .set_name(format!("projects/{project_id}"))
        .set_interval(TimeInterval::new().set_end_time(end).set_start_time(start))
        .set_filter(format!(
            r#"metric.type = "{metric_name}" AND metric.label.{key} = "{value}""#
        ))
        .send()
        .await?;
    Ok(Some(response))
}

/// Sets up logs, metrics and tracing providers sending the signals to Google Cloud.
///
/// This function configures global OpenTelemetry providers that send logs,
/// metrics and traces to Cloud Logging, Cloud Monitoring, and Cloud Trace,
/// respectively.
///
/// The providers all use the OTLP endpoint (`telemetry.googleapis.com`) and the
/// gRPC-based protocol.
///
/// All the tests in a process use the same provider.
pub async fn set_up_providers(project_id: &str) -> anyhow::Result<&Providers> {
    PROVIDERS
        .get_or_init(|| self::new_providers(project_id))
        .await
        // `get_or_init()` returns a `&Result<T>` so we need some mapping.
        .as_ref()
        // Cannot clone anyhow::Error, so do this instead:
        .map_err(|e| anyhow::anyhow!("badly initialized provider: {e:?}"))
}

/// Creates a new tracer provider for the tests.
///
/// This uses ADC, and configures a quota project for user credentials because
/// telemetry endpoint rejects user credentials without the quota user project.
///
/// Note that some other services reject requests *with* a quota user project.
/// Therefore, we cannot require that the credentials have a quota user project
/// set.
async fn new_providers(project_id: &str) -> anyhow::Result<Providers> {
    let detector = TestResourceDetector::new(project_id);
    let credentials = new_credentials(project_id).await?;
    let trace = TracerProviderBuilder::new(project_id, SERVICE_NAME)
        .with_credentials(credentials.clone())
        .with_detector(detector.clone())
        .build()
        .await?;
    let metrics = MeterProviderBuilder::new(project_id, SERVICE_NAME)
        .with_credentials(credentials.clone())
        .with_detector(detector.clone())
        .build()
        .await?;
    let client = LoggingServiceV2::builder()
        .with_credentials(credentials)
        .build()
        .await?;
    let logs = LoggerProviderBuilder::new(project_id, SERVICE_NAME)
        .with_client(client)
        .with_detector(detector)
        .build()
        .await?;

    // Install subscriber, ignore any other subscriber already installed.
    if let Err(e) = tracing::subscriber::set_global_default(
        tracing_subscriber::Registry::default()
            // Capture traces, automatically include logs ...
            .with(super::tracing::layer(trace.clone()))
            // ... send the trace events to the logger provider ...
            .with(
                OpenTelemetryTracingBridge::new(&logs)
                    .with_filter(LevelFilter::from_level(Level::INFO)),
            )
            // ... print things, which is useful to troubleshoot the tests.
            .with(
                tracing_subscriber::fmt::layer()
                    .with_filter(tracing_subscriber::EnvFilter::from_default_env()),
            ),
    ) {
        tracing::error!("Cannot set global default for tracing subscriber: {e:?}");
    }
    // Install the provider, making it available to tests and the client
    // libraries.
    opentelemetry::global::set_meter_provider(metrics.clone());

    Ok(Providers {
        trace,
        metrics,
        logs,
    })
}

async fn new_credentials(project_id: &str) -> anyhow::Result<Credentials> {
    let credentials = CredentialsBuilder::default().build()?;
    let credentials = if format!("{credentials:?}").contains("UserCredentials") {
        CredentialsBuilder::default()
            .with_quota_project_id(project_id)
            .build()?
    } else {
        credentials
    };
    Ok(credentials)
}
