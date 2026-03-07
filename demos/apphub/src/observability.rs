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

use super::Args;
use super::logs::EventFormatter;
use google_cloud_auth::credentials::Credentials;
use integration_tests_o11y::detector::GoogleCloudResourceDetector;
use integration_tests_o11y::otlp::metrics::Builder as MeterProviderBuilder;
use integration_tests_o11y::otlp::trace::Builder as TracerProviderBuilder;
use integration_tests_o11y::tracing::layer as otlp_layer;
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::format::FmtSpan;
use uuid::Uuid;

/// Configure exporters for traces, logs, and metrics.
pub async fn exporters(args: &Args, credentials: Credentials) -> anyhow::Result<()> {
    use tracing_subscriber::prelude::*;

    let logging_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::NONE)
        .with_level(true)
        .with_thread_ids(true)
        .event_format(EventFormatter)
        .with_filter(LevelFilter::INFO);

    let id = Uuid::new_v4();
    let development = Resource::builder_empty()
        .with_attributes([
            KeyValue::new("location", "us-central1"),
            KeyValue::new("namespace", "google-cloud-rust"),
            KeyValue::new("node_id", id.to_string()),
        ])
        .build();
    let detector = GoogleCloudResourceDetector::builder()
        .with_fallback(development)
        .build()
        .await?;
    if args.project_id.is_empty() || args.service_name.is_empty() {
        tracing::subscriber::set_global_default(Registry::default().with(logging_layer))?;
        return Ok(());
    }
    let project_id = &args.project_id;
    let service = &args.service_name;
    let tracer_provider = TracerProviderBuilder::new(project_id, service)
        .with_credentials(credentials.clone())
        .with_detector(detector.clone())
        .build()
        .await?;
    let meter_provider = MeterProviderBuilder::new(project_id, service)
        .with_credentials(credentials.clone())
        .with_detector(detector.clone())
        .build()
        .await?;

    tracing::subscriber::set_global_default(
        Registry::default()
            .with(logging_layer)
            .with(otlp_layer(tracer_provider.clone())),
    )?;
    opentelemetry::global::set_meter_provider(meter_provider.clone());
    Ok(())
}
