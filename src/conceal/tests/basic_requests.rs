// Copyright 2024 Google LLC
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

use conceal::traits::FooService as _;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn basic_requests() -> Result<()> {
    let (endpoint, _server) = conceal::server::start().await?;
    let client = conceal::client::FooService::new(&endpoint).await?;

    let response = client
        .list_foos()
        .set_prefix("abc")
        .with_timeout(std::time::Duration::from_millis(100))
        .send()
        .await;

    println!("response = {response:?}");

    Ok(())
}
