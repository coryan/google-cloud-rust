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

pub struct ClientOptions {
    pub client_builder: reqwest::ClientBuilder,
    pub endpoint: Option<String>,
    pub credentials: Option<Box<dyn crate::tokens::AccessTokenSource + Send + Sync>>,
}

impl ClientOptions {
    pub fn default() -> Self {
        Self {
            client_builder: reqwest::Client::builder(),
            endpoint: None,
            credentials: None
        }
    }

    pub fn set_enpoint<T: Into<Option<String>>>(mut self, v: T) -> Self {
        self.endpoint = v.into();
        self
    }
}
