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

use integration_tests::traits_prototype::*;
use integration_tests::traits_prototype::model::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn use_generic_traits() {
    let svc = Builder::build();
    let svc = Builder::with_retry(3, svc);
    let svc = Builder::with_tracing(svc);

    let response = svc.rpc(Request{ parent: "parent".to_string(), id: "id".to_string()}).await.unwrap();
    assert_eq!(response.name, "parent/foos/id");
}
