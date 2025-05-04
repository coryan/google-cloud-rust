# Copyright 2025 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM rust:1.85-bookworm as build
WORKDIR /w
COPY Cargo.toml /w
RUN mkdir /w/src && touch /w/src/main.rs
RUN cargo build --release -p google-cloud-secretmanager-v1 -p google-cloud-iam-admin-v1 -p axum -p anyhow
COPY src/main.rs /w/src/main.rs
RUN cargo build --release -p key-rotation

FROM debian:bookworm
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=build /w/target/release/key-rotation /r/key-rotation
EXPOSE 8080
CMD ["r/key-rotation"]
