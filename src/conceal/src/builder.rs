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

use super::model;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RequestBuilder<T: std::default::Default> {
    request: T,
    options: HashMap<String, String>,
    stub: Arc<dyn crate::stubs::dyncompatible::FooService>,
}

impl<T> RequestBuilder<T>
where T: std::default::Default {
    pub(crate) fn new(stub: Arc<dyn crate::stubs::dyncompatible::FooService>) -> Self {
        Self {
            request: T::default(),
            // Should provide defaults for timeout and retry options.
            options: HashMap::default(),
            stub,
        }
    }

    /// Set the full request.
    pub fn with_request<V: Into<T>>(mut self, v: V) -> Self {
        self.request = v.into();
        self
    }

    /// Set the timeout option.
    pub fn with_timeout<V: Into<std::time::Duration>>(mut self, v: V) -> Self {
        let d: std::time::Duration = v.into();
        self.options.insert("timeout".into(), format!("{:?}", d));
        self
    }

    /// Set the user agent option.
    pub fn with_user_agent<V: Into<String>>(mut self, v: V) -> Self {
        self.options.insert("user-agent".into(), v.into());
        self
    }
}

pub type ListFoosRequest = RequestBuilder<model::ListFoosRequest>;

impl ListFoosRequest {
    /// Only return the `Foos` starting with this prefix.
    pub fn set_prefix<T: Into<String>>(mut self, v: T) -> Self {
        self.request.prefix = v.into();
        self
    }

    pub async fn send(self) -> Result<model::ListFoosResponse> {
        self.stub.list_foos(self.request, self.options).await
    }

    pub fn stream(self) -> gax::paginator::Paginator<model::ListFoosResponse, gax::error::Error> {
        let token = self.request.page_token.clone();
        let execute = move |token: String| {
            let mut copy = self.clone();
            copy.request.page_token = token;
            copy.send()
        };
        gax::paginator::Paginator::new(token, execute)
    }
}

pub type CreateFooRequest = RequestBuilder<model::CreateFooRequest>;

impl CreateFooRequest {
    /// Set the parent.
    pub fn set_parent<T: Into<String>>(mut self, v: T) -> Self {
        self.request.parent = v.into();
        self
    }

    /// Set the item id.
    pub fn set_foo_id<T: Into<String>>(mut self, v: T) -> Self {
        self.request.foo_id = v.into();
        self
    }

    pub fn set_item<T: Into<model::Foo>>(mut self, v: T) -> Self {
        self.request.item = v.into();
        self
    }

    pub async fn send(self) -> Result<model::Foo> {
        self.stub.create_foo(self.request, self.options).await
    }
}

pub struct GetFooRequest;

pub struct DeleteFooRequest;

