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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
