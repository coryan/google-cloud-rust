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


#[cfg(feature = "unstable-dyntraits")]
#[cfg(test)]
mod mocking {
    use integration_tests::wrapped_inner_client::model;
    use integration_tests::wrapped_inner_client::builder;
    use integration_tests::wrapped_inner_client::dyntraits;
    
    use gax::error::Error;
    type Result<T> = std::result::Result<T, Error>;

    async fn application(svc: impl dyntraits::FooService) -> Result<Vec<String>> {
        let mut result = Vec::new();
        for id in ["id0", "id1", "id2"] {
            let r = svc
                .create_foo(model::CreateFooRequest {
                    parent: "p".to_string(),
                    id: id.to_string(),
                    body: model::Foo::default(),
                })
                .await?;
            result.push(r);
        }
        let result = result.into_iter().map(|r| r.name).collect();
        Ok(result)
    }

    mockall::mock! {
        ServiceImpl {}
        #[async_trait::async_trait]
        impl dyntraits::FooService for ServiceImpl {
            async fn create_foo(&self, req: model::CreateFooRequest) -> Result<model::Foo>;
        }
    }

    impl std::fmt::Debug for MockServiceImpl {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            write!(f, "Mock")
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn mock() {
        let mut svc = MockServiceImpl::new();
        svc.expect_create_foo()
            .return_once(|_| Err(Error::other("simulated failure")));

        let result = application(svc).await;
        assert!(result.is_err());
        let error = result.err().unwrap();
        match error.kind() {
            gax::error::ErrorKind::Other => {}
            _ => {
                assert!(false, "unexpected error type");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn for_real() -> Result<()> {
        let svc = builder::FooService::new().build(); 

        let result = application(svc).await?;
        assert_eq!(
            result,
            ["p/foos/id0", "p/foos/id1", "p/foos/id2"]
                .map(str::to_string)
                .to_vec()
        );
        Ok(())
    }
}
