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

/// A `Foo`. The resource managed by `FooService`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Foo {
    pub name: String,
    pub payload: String,
}

/// The request message for [super::traits::FooService::list_foos].
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ListFoosRequest {
    pub prefix: String,
    pub page_size: Option<i32>,
    pub page_token: String,
}

/// The response message for [super::traits::FooService::list_foos].
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ListFoosResponse {
    pub items: Vec<Foo>,
    pub next_page_token: Option<String>,
}

impl gax::paginator::PageableResponse for ListFoosResponse {
    fn next_page_token(&self) -> String {
        self.next_page_token
            .as_ref()
            .cloned()
            .unwrap_or_else(String::new)
    }
}

/// The request message for [super::traits::FooService::create_foo].
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateFooRequest {
    pub parent: String,
    pub foo_id: String,
    pub item: Foo
}
