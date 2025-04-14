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

use mscope::Poller;
use crate::Result;

pub async fn run(builder: mscope::builder::metrics_scopes::ClientBuilder) -> Result<()> {
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
    let client = builder.build().await?;

    let name = format!("projects/{project_id}");
    let response = client.list_metrics_scopes_by_monitored_project().set_monitored_resource_container(&name).send().await?;
    response.metrics_scopes.into_iter().for_each(|item| println!("  ITEM={item:?}"));

    let parent = format!("locations/global/metricsScopes/{project_id}");
    let name = format!("{parent}/projects/rust-sdk-testing");
    let create = client.create_monitored_project(&parent).set_monitored_project(mscope::model::MonitoredProject::new().set_name(&name)).poller().until_done().await?;
    println!("CREATE = {create:?}");


    Ok(())
}
