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

use crate::Result;
use gax::exponential_backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use gax::paginator::ItemPaginator as _;
use gax::{error::Error, options::RequestOptionsBuilder};
use std::time::Duration;
use wf::Poller;

pub async fn until_done(builder: wf::builder::workflows::ClientBuilder) -> Result<()> {
    // Enable a basic subscriber. Useful to troubleshoot problems and visually
    // verify tracing is doing something.
    #[cfg(feature = "log-integration-tests")]
    let _guard = {
        use tracing_subscriber::fmt::format::FmtSpan;
        let subscriber = tracing_subscriber::fmt()
            .with_level(true)
            .with_thread_ids(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish();

        tracing::subscriber::set_default(subscriber)
    };

    let project_id = crate::project_id()?;
    let location_id = crate::region_id();
    let workflows_runner = crate::workflows_runner()?;

    let client = builder.build().await?;
    cleanup_stale_workflows(&client, &project_id, &location_id).await?;

    let source_contents = r###"# Test only workflow
main:
    steps:
        - sayHello:
            return: Hello World
"###;
    let source_code = wf::model::workflow::SourceCode::SourceContents(source_contents.to_string());
    let workflow_id = crate::random_workflow_id();

    tracing::info!("Start create_workflow() LRO and poll it to completion");
    let response = client
        .create_workflow(format!("projects/{project_id}/locations/{location_id}"))
        .set_workflow_id(&workflow_id)
        .set_workflow(
            wf::model::Workflow::new()
                .set_labels([("integration-test", "true")])
                .set_service_account(&workflows_runner)
                .set_source_code(source_code),
        )
        .with_polling_backoff_policy(test_backoff()?)
        .poller()
        .until_done()
        .await?;
    tracing::info!("create LRO finished, response={response:?}");

    tracing::info!("Start delete_workflow() LRO and poll it to completion");
    client
        .delete_workflow(format!(
            "projects/{project_id}/locations/{location_id}/workflows/{workflow_id}"
        ))
        .poller()
        .until_done()
        .await?;
    tracing::info!("delete LRO finished");

    Ok(())
}

pub async fn explicit_loop(builder: wf::builder::workflows::ClientBuilder) -> Result<()> {
    // Enable a basic subscriber. Useful to troubleshoot problems and visually
    // verify tracing is doing something.
    #[cfg(feature = "log-integration-tests")]
    let _guard = {
        use tracing_subscriber::fmt::format::FmtSpan;
        let subscriber = tracing_subscriber::fmt()
            .with_level(true)
            .with_thread_ids(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish();

        tracing::subscriber::set_default(subscriber)
    };

    let project_id = crate::project_id()?;
    let location_id = crate::region_id();
    let workflows_runner = crate::workflows_runner()?;

    let client = builder.build().await?;
    cleanup_stale_workflows(&client, &project_id, &location_id).await?;

    let source_contents = r###"# Test only workflow
main:
    steps:
        - sayHello:
            return: Hello World
"###;
    let source_code = wf::model::workflow::SourceCode::SourceContents(source_contents.to_string());
    let workflow_id = crate::random_workflow_id();

    tracing::info!("Start create_workflow() LRO and poll it to completion");
    let mut create = client
        .create_workflow(format!("projects/{project_id}/locations/{location_id}"))
        .set_workflow_id(&workflow_id)
        .set_workflow(
            wf::model::Workflow::new()
                .set_labels([("integration-test", "true")])
                .set_service_account(&workflows_runner)
                .set_source_code(source_code),
        )
        .poller();
    let mut backoff = Duration::from_millis(100);
    while let Some(status) = create.poll().await {
        match status {
            wf::PollingResult::PollingError(e) => {
                tracing::info!("error polling create LRO, continuing {e}");
            }
            wf::PollingResult::InProgress(m) => {
                tracing::info!("create LRO still in progress, metadata={m:?}");
            }
            wf::PollingResult::Completed(r) => match r {
                Err(e) => {
                    tracing::info!("create LRO finished with error={e}");
                    return Err(e);
                }
                Ok(m) => {
                    tracing::info!("create LRO finished with success={m:?}");
                }
            },
        }
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
    }

    tracing::info!("Start delete_workflow() LRO and poll it to completion");
    let mut delete = client
        .delete_workflow(format!(
            "projects/{project_id}/locations/{location_id}/workflows/{workflow_id}"
        ))
        .poller();
    let mut backoff = Duration::from_millis(100);
    while let Some(status) = delete.poll().await {
        match status {
            wf::PollingResult::PollingError(e) => {
                tracing::info!("error polling delete LRO, continuing {e:?}");
            }
            wf::PollingResult::InProgress(m) => {
                tracing::info!("delete LRO still in progress, metadata={m:?}");
            }
            wf::PollingResult::Completed(Ok(_)) => {
                println!("    delete LRO finished successfully");
            }
            wf::PollingResult::Completed(Err(e)) => {
                println!("    delete LRO finished with an error {e}");
                return Err(e);
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
    }

    Ok(())
}

pub async fn suspend_and_resume(builder: wf::builder::workflows::ClientBuilder) -> Result<()> {
    // Enable a basic subscriber. Useful to troubleshoot problems and visually
    // verify tracing is doing something.
    #[cfg(feature = "log-integration-tests")]
    let _guard = {
        use tracing_subscriber::fmt::format::FmtSpan;
        let subscriber = tracing_subscriber::fmt()
            .with_level(true)
            .with_thread_ids(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish();

        tracing::subscriber::set_default(subscriber)
    };

    let project_id = crate::project_id()?;
    let location_id = crate::region_id();
    let workflows_runner = crate::workflows_runner()?;

    let client = builder.build().await?;
    cleanup_stale_workflows(&client, &project_id, &location_id).await?;

    let source_contents = r###"# Test only workflow
main:
    steps:
        - sayHello:
            return: Hello World
"###;
    let source_code = wf::model::workflow::SourceCode::SourceContents(source_contents.to_string());
    let workflow_id = crate::random_workflow_id();

    tracing::info!("Start create_workflow() LRO and poll it to completion");
    let mut create = client
        .create_workflow(format!("projects/{project_id}/locations/{location_id}"))
        .set_workflow_id(&workflow_id)
        .set_workflow(
            wf::model::Workflow::new()
                .set_labels([("integration-test", "true")])
                .set_service_account(&workflows_runner)
                .set_source_code(source_code),
        )
        .poller();
    let mut backoff = Duration::from_millis(100);
    let mut snapshot = None;
    while let Some(status) = create.poll().await {
        match status {
            wf::PollingResult::PollingError(e) => {
                tracing::info!("error polling create LRO, continuing {e}");
                snapshot = create.suspend();
                break;
            }
            wf::PollingResult::InProgress(m) => {
                tracing::info!("create LRO still in progress, metadata={m:?}");
                snapshot = create.suspend();
                break;
            }
            wf::PollingResult::Completed(Err(e)) => {
                tracing::info!("create LRO finished with error={e}");
                return Err(e);
            }
            wf::PollingResult::Completed(Ok(w)) => {
                tracing::info!("create LRO finished with success={w:?}");
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
    }

    if let Some(s) = snapshot {
        let w = client.resume_poller(s).until_done().await;
        tracing::info!("create LRO completed after resume success={w:?}");
    }

    tracing::info!("Start delete_workflow() LRO and poll it to completion");
    let mut delete = client
        .delete_workflow(format!(
            "projects/{project_id}/locations/{location_id}/workflows/{workflow_id}"
        ))
        .poller();
    let mut backoff = Duration::from_millis(100);
    while let Some(status) = delete.poll().await {
        match status {
            wf::PollingResult::PollingError(e) => {
                tracing::info!("error polling delete LRO, continuing {e:?}");
            }
            wf::PollingResult::InProgress(m) => {
                tracing::info!("delete LRO still in progress, metadata={m:?}");
            }
            wf::PollingResult::Completed(r) => {
                tracing::info!("delete LRO finished, result={r:?}");
                let _ = r?;
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
    }

    Ok(())
}

fn test_backoff() -> Result<ExponentialBackoff> {
    ExponentialBackoffBuilder::new()
        .with_initial_delay(Duration::from_millis(100))
        .with_maximum_delay(Duration::from_secs(1))
        .build()
}

async fn cleanup_stale_workflows(
    client: &wf::client::Workflows,
    project_id: &str,
    location_id: &str,
) -> Result<()> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let stale_deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(Error::other)?;
    let stale_deadline = stale_deadline - Duration::from_secs(48 * 60 * 60);
    let stale_deadline = wkt::Timestamp::clamp(stale_deadline.as_secs() as i64, 0);

    let mut paginator = client
        .list_workflows(format!("projects/{project_id}/locations/{location_id}"))
        .by_item();
    let mut stale_workflows = Vec::new();
    while let Some(workflow) = paginator.next().await {
        let item = workflow?;
        if let Some("true") = item.labels.get("integration-test").map(String::as_str) {
            if let Some(true) = item.create_time.map(|v| v < stale_deadline) {
                stale_workflows.push(item.name);
            }
        }
    }
    let pending = stale_workflows
        .iter()
        .map(|name| client.delete_workflow(name).poller().until_done())
        .collect::<Vec<_>>();

    futures::future::join_all(pending)
        .await
        .into_iter()
        .zip(stale_workflows)
        .for_each(|(r, name)| tracing::info!("{name} = {r:?}"));

    Ok(())
}

pub async fn manual(
    project_id: String,
    region_id: String,
    workflow_id: String,
    workflow: wf::model::Workflow,
) -> Result<()> {
    let client = wf::client::Workflows::builder().build().await?;

    tracing::info!("Start create_workflow() LRO and poll it to completion");
    let create = client
        .create_workflow(format!("projects/{project_id}/locations/{region_id}"))
        .set_workflow_id(&workflow_id)
        .set_workflow(workflow)
        .send()
        .await?;
    if create.done {
        use longrunning::model::operation::Result as LR;
        let result = create
            .result
            .ok_or("service error: done with missing result ")
            .map_err(Error::other)?;
        match result {
            LR::Error(status) => {
                tracing::info!("LRO completed with error {status:?}");
                let err = gax::error::ServiceError::from(*status);
                return Err(Error::rpc(err));
            }
            LR::Response(any) => {
                tracing::info!("LRO completed successfully {any:?}");
                let response = any
                    .try_into_message::<wf::model::Workflow>()
                    .map_err(Error::other);
                tracing::info!("LRO completed response={response:?}");
                return Ok(());
            }
            _ => panic!("unexpected branch"),
        }
    }
    let name = create.name;
    loop {
        let operation = client.get_operation(name.clone()).send().await?;
        if !operation.done {
            tracing::info!("operation is pending {operation:?}");
            if let Some(any) = operation.metadata {
                match any.try_into_message::<wf::model::OperationMetadata>() {
                    Err(_) => {
                        tracing::info!("cannot extract expected metadata from {any:?}");
                    }
                    Ok(metadata) => {
                        tracing::info!("metadata={metadata:?}");
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        use longrunning::model::operation::Result as LR;
        let result = create
            .result
            .ok_or("service error: done with missing result ")
            .map_err(Error::other)?;
        match result {
            LR::Error(status) => {
                tracing::info!("LRO completed with error {status:?}");
                let err = gax::error::ServiceError::from(*status);
                return Err(Error::rpc(err));
            }
            LR::Response(any) => {
                tracing::info!("LRO completed successfully {any:?}");
                let response = any
                    .try_into_message::<wf::model::Workflow>()
                    .map_err(Error::other);
                tracing::info!("LRO completed response={response:?}");
                return Ok(());
            }
            _ => panic!("unexpected branch"),
        }
    }
}
