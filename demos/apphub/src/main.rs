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

//! Shows how to deploy Rust applications to Cloud Run and monitor them with AppHub.

const DESCRIPTION: &str = concat!(
    "The demo highlights how Rust applications can be monitored using Google Cloud AppHub.",
    "",
    "This application runs a web application. The application presents the user with a simple UI",
    " to enter a question, maybe with links to images in Cloud Storage. The application reads this",
    " prompt, sends a request to Gemini (via Vertex AI) based on the prompt, and then presents the",
    " response to the user.",
    "",
    "Each request to Cloud Storage and Vertex AI are traced, their latency is measured, and any",
    " errors are logged in a format that Cloud Logging can consume. The changes to support this",
    " logging are found in a single function, with minimal impact on the application code or the",
    " initialization of the client libraries."
);

mod args;
mod error;
mod observability;
mod state;

use args::Args;
use axum::Router;
use axum::extract::State;
use axum::routing;
use clap::Parser;
use error::AppError;
use state::AppState;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::only_logs()?;
    tracing::info!(key0 = "value0", key1 = "value1", "info 123");
    tracing::warn!(key0 = "value0", key1 = "value1", "warn 234");
    {
        let _span = tracing::info_span!("blah blah", key2 = "v2").entered();
        tracing::info!("in span");
    }

    let args = Args::parse();
    tracing::info!("Initial configuration: {args:?}");

    let state = AppState::new(args.clone()).await?;

    let app = Router::new()
        .route("/", routing::get(handler))
        .route("/fortune", routing::get(predict))
        .with_state(state);
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler() -> &'static str {
    "Hello, world!\n"
}

async fn predict(State(state): State<AppState>) -> anyhow::Result<String, AppError> {
    use google_cloud_aiplatform_v1::model::{Content, FileData, Part};

    const MODEL: &str = "gemini-2.5-flash";
    let model = format!(
        "projects/{}/locations/global/publishers/google/models/{MODEL}",
        state.args().project_id
    );
    let response = state
        .prediction_service()
        .generate_content()
        .set_model(&model)
        .set_contents([Content::new().set_role("user").set_parts([
            // [START rust_prompt_and_image_image_part] ANCHOR: prompt-and-image-image-part
            Part::new().set_file_data(
                FileData::new()
                    .set_mime_type("image/jpeg")
                    .set_file_uri("gs://generativeai-downloads/images/scones.jpg"),
            ),
            // [END rust_prompt_and_image_image_part] ANCHOR_END: prompt-and-image-image-part
            // [START rust_prompt_and_image_prompt_part] ANCHOR: prompt-and-image-prompt-part
            Part::new().set_text("Describe this picture."),
            // [END rust_prompt_and_image_prompt_part] ANCHOR_END: prompt-and-image-prompt-part
        ])])
        .send()
        .await?;

    Ok(format!("{response:#?}"))
}
