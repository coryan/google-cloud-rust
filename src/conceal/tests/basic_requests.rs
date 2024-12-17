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
async fn basic_list() -> Result<()> {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streaming_list() -> Result<()> {
    let (endpoint, _server) = conceal::server::start().await?;
    let client = conceal::client::FooService::new(&endpoint).await?;

    let mut stream = client
        .list_foos()
        .set_prefix("abc")
        .with_timeout(std::time::Duration::from_millis(100))
        .stream();

    while let Some(page) = stream.next().await {
        let page = page?;
        println!("page = {page:?}");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_and_list() -> Result<()> {
    let (endpoint, _server) = conceal::server::start().await?;
    let client = conceal::client::FooService::new(&endpoint).await?;

    let response = client
        .create_foo()
        .set_parent("abc")
        .set_foo_id("123")
        .with_timeout(std::time::Duration::from_millis(100))
        .send()
        .await?;

    println!("response1 = {response:?}");
    let response = client
        .create_foo()
        .set_parent("abc")
        .set_foo_id("234")
        .with_timeout(std::time::Duration::from_millis(100))
        .send()
        .await?;

    println!("response2 = {response:?}");

    let response = client
        .list_foos()
        .set_prefix("abc")
        .with_timeout(std::time::Duration::from_millis(100))
        .send()
        .await;

    println!("response = {response:?}");

    Ok(())
}
