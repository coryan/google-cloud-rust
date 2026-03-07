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

use clap::Parser;

/// Command-line arguments.
#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = super::DESCRIPTION)]
struct Args {
    /// The name of the bucket used by the application.
    #[arg(long)]
    bucket_name: String,

    /// The default project name, if not found via resource discovery.
    #[arg(long, env = "GOOGLE_CLOUD_PROJECT")]
    project_id: String,

    /// The default project name, if not found via resource discovery.
    #[arg(long, env = "K_SERVICE")]
    service_name: String,
}
