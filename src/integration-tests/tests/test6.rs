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

use integration_tests::client6::model::*;
use integration_tests::client6::*;

use gax::error::Error;
type Result<T> = std::result::Result<T, Error>;

async fn application(svc: impl traits::FooService) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for id in ["id0", "id1", "id2"] {
        let r = svc
            .create_foo(CreateFooRequest {
                parent: "test-only".to_string(),
                id: id.to_string(),
                body: Foo::default(),
            })
            .await?;
        result.push(r);
    }
    let result = result.into_iter().map(|r| r.name).collect();
    Ok(result)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn basic_usage() -> Result<()> {
    let client = builder::FooService::default().with_tracing().build();

    let result = application(client).await?;
    println!("{result:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn fully_static() -> Result<()> {
    let client = builder::StaticBuilder::default().build_with_tracing();

    let result = application(client).await?;
    println!("{result:?}");
    Ok(())
}

#[cfg(feature = "unstable-dyntraits")]
mod if_you_insist_you_can_use_dyn {
    use super::*;

    async fn dynapplication(svc: &dyn dyntraits::FooService) -> Result<Vec<String>> {
        let mut result = Vec::new();
        for id in ["id0", "id1", "id2"] {
            let r = svc
                .create_foo(CreateFooRequest {
                    parent: "test-only".to_string(),
                    id: id.to_string(),
                    body: Foo::default(),
                })
                .await?;
            result.push(r);
        }
        let result = result.into_iter().map(|r| r.name).collect();
        Ok(result)
    }
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn with_arc() -> Result<()> {
        use std::sync::Arc;
        let client: Arc<dyn dyntraits::FooService> =
            Arc::new(builder::StaticBuilder::default().build());
    
        let result = dynapplication(client.as_ref()).await?;
        println!("{result:?}");
        Ok(())
    }    
}

mod mocking {
    use super::*;

    mockall::mock! {
        #[derive(Debug)]
        FooService {}
        impl traits::FooService for FooService {
            async fn create_foo(&self, req: model::CreateFooRequest) -> Result<model::Foo>;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn mock() -> Result<()> {
        let mut mock = MockFooService::new();
        mock.expect_create_foo()
            .return_once(|_| Err(Error::other("simulated failure")));

        let response = application(mock).await;
        assert!(response.is_err());
        Ok(())
    }
}
