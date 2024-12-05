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


#[cfg(test)]
mod tracing_test {
    use integration_tests::wrapped_inner_client::{builder, traits, model};
    
    use gax::error::Error;
    type Result<T> = std::result::Result<T, Error>;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn for_real() -> Result<()> {
        use traits::FooService;
        let svc = builder::FooService::new().with_tracing().build(); 

        let mut result = Vec::new();
        for id in ["id0", "id1", "id2"] {
            let r = svc
                .create_foo(model::CreateFooRequest {
                    parent: "test-only".to_string(),
                    id: id.to_string(),
                    body: model::Foo::default(),
                })
                .await?;
            result.push(r);
        }

        let result : Vec<_> = result.into_iter().map(|r| r.name).collect();
        assert_eq!(
            result,
            ["test-only/foos/id0", "test-only/foos/id1", "test-only/foos/id2"]
                .map(str::to_string)
                .to_vec()
        );
        Ok(())
    }

}
