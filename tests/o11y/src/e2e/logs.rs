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

use super::{set_up_providers, wait_for_trace};
use google_cloud_logging_type::model::LogSeverity;
use google_cloud_test_utils::runtime_config::project_id;
use opentelemetry::trace::TraceContextExt;
use std::time::Duration;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use google_cloud_logging_v2::client::LoggingServiceV2;

const ROOT_SPAN_NAME: &str = "e2e-logs-test";

pub async fn run() -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let project_id = project_id()?;
    let providers = set_up_providers(&project_id).await?;

    let root_span = tracing::info_span!("e2e_root", "otel.name" = ROOT_SPAN_NAME);
    let trace_id = root_span
        .context()
        .span()
        .span_context()
        .trace_id()
        .to_string();

    tracing::event!(Level::DEBUG, testId = id, "some debugging help");
    tracing::event!(Level::INFO, testId = id, "some informational message");
    tracing::event!(Level::WARN, testId = id, "a warning about things");
    tracing::event!(Level::ERROR, testId = id, "a real error");
    {
        let _enter = root_span.enter();
        tracing::event!(Level::ERROR, testId = id, "an error inside a span");
    }
    drop(root_span);
    println!(
        "View generated trace in Console: https://console.cloud.google.com/traces/explorer;traceId={trace_id}?project={project_id}",
    );

    // 4. Force flush to ensure spans are sent.
    providers.trace.force_flush()?;

    // 5. Verify (Poll Cloud Trace API)
    let _trace = wait_for_trace(&project_id, &trace_id).await?;

    let client = LoggingServiceV2::builder().build().await?;

    // This may take several attempts, as inserting values in a timeseries is rate limited.
    let mut found = None;
    for delay in [0, 10 /* , 60, 120, 120, 120*/].map(Duration::from_secs) {
        tokio::time::sleep(delay).await;
        // Ignore errors because it may have flushed recently.
        if let Err(e) = providers.trace.force_flush() {
            tracing::error!("error flushing traces: {e:?}");
        }
        found = try_get_log(&client, &project_id, ("testId", id.as_str())).await?;
        // Wait until all 4 logs appear in the service.
        if found.as_ref().is_some_and(|f| f.entries.len() >= 4) {
            break;
        }
    }

    let found = found.expect("log entries are found in Cloud Logging");
    // The order of the entries is not predictable, we need to search in them.
    let entries = match &found.entries[..] {
        [i, w, e, es] => [i, w, e, es],
        _ => panic!("expected 4 log messages, found: {found:#?}"),
    };

    let info = entries
        .into_iter()
        .find(|e| e.severity == LogSeverity::Info)
        .expect("at least one INFO message");
    assert!(
        info.resource
            .as_ref()
            .is_some_and(|r| r.r#type == "generic_node"),
        "{info:?}"
    );

    let warn = entries
        .into_iter()
        .find(|e| e.severity == LogSeverity::Warning)
        .expect("at least one WARNING message");
    assert!(
        warn.resource
            .as_ref()
            .is_some_and(|r| r.r#type == "generic_node"),
        "{warn:?}"
    );
    let error = entries
        .into_iter()
        .find(|e| e.severity == LogSeverity::Error && e.trace.is_empty())
        .expect("at least one ERROR message without tracing");
    assert!(
        error
            .resource
            .as_ref()
            .is_some_and(|r| r.r#type == "generic_node"),
        "{warn:?}"
    );

    let is_span = entries
        .into_iter()
        .find(|e| e.severity == LogSeverity::Error && !e.trace.is_empty())
        .expect("at least one ERROR message with tracing");
    assert!(
        is_span
            .resource
            .as_ref()
            .is_some_and(|r| r.r#type == "generic_node"),
        "{warn:?}"
    );

    Ok(())
}

use google_cloud_logging_v2::model::ListLogEntriesResponse;

pub async fn try_get_log(
    client: &LoggingServiceV2,
    project_id: &str,
    label: (&str, &str),
) -> anyhow::Result<Option<ListLogEntriesResponse>> {
    use google_cloud_wkt::Timestamp;
    use std::time::SystemTime;
    let _end = Timestamp::try_from(SystemTime::now())?;
    let start = Timestamp::try_from(SystemTime::now() - Duration::from_secs(300))?;
    let (key, value) = label;
    let response = client
        .list_log_entries()
        .set_resource_names([format!("projects/{project_id}")])
        .set_filter(format!(
            r#"labels.{key} = "{value}" AND timestamp >= "{}""#,
            String::from(start),
        ))
        .set_order_by("timestamp desc")
        .send()
        .await?;
    Ok(Some(response))
}
