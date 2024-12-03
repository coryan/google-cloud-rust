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

use integration_tests::traits_prototype::model::*;
use integration_tests::traits_prototype::*;

use gax::error::Error;
type Result<T> = std::result::Result<T, Error>;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn use_generic_traits() {
    let svc = Builder::build();
    let svc = Builder::with_retry(3, svc);
    let svc = Builder::with_tracing(svc);

    let response = svc
        .rpc(Request {
            parent: "parent".to_string(),
            id: "id".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(response.name, "parent/foos/id");
}

#[cfg(test)]
mod test {
    use super::*;

    async fn application(svc: impl Service) -> Result<Vec<String>> {
        let mut result = Vec::new();
        for id in ["id0", "id1", "id2"] {
            let r = svc
                .rpc(Request {
                    parent: "p".to_string(),
                    id: id.to_string(),
                })
                .await?;
            result.push(r);
        }
        let result = result.into_iter().map(|r| r.name).collect();
        Ok(result)
    }

    mockall::mock! {
        ServiceImpl {}
        impl Service for ServiceImpl {
            async fn rpc(&self, req: Request) -> Result<Response>;
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
        svc.expect_rpc()
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
        let svc = Builder::build();
        let svc = Builder::with_retry(3, svc);
        let svc = Builder::with_tracing(svc);

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
