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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "refresh-protos")]
    {
        let root = std::env::var("GOOGLEAPIS_ROOT")?;
        let protos = [
            "google/bigtable/v2/bigtable.proto",
            "google/bigtable/v2/data.proto",
            "google/bigtable/v2/feature_flags.proto",
            "google/bigtable/v2/request_stats.proto",
            "google/bigtable/v2/response_params.proto",
            "google/bigtable/v2/types.proto",
        ].map(|v| std::path::Path::new(&root).join(v)).to_vec();
    
        let mut config = prost_build::Config::new();
        config.bytes(&["."]);
        tonic_build::configure()
            .build_server(false)
            .out_dir("src/generated")
            .compile_protos_with_config(
                config,
                &protos.iter().filter_map(|p| p.to_str()).collect::<Vec<&str>>(),
                &[&root],
            )?;    
    }
    Ok(())
}
