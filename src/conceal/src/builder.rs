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
use crate::Error;
use crate::Result;
use std::collections::HashMap;

type HttpClient = gax::http_client::ReqwestClient;

#[derive(Clone, Debug)]
pub struct ListFoosRequest {
    request: model::ListFoosRequest,

    // Placeholder, this should be a real implementation.
    options: HashMap<String, String>,

    client: HttpClient,
}

impl ListFoosRequest {
    pub fn new(client: HttpClient) -> Self {
        Self {
            request: model::ListFoosRequest::default(),
            // Should provide defaults for timeout and retry options.
            options: HashMap::default(),
            client,
        }
    }

    /// Only return the `Foos` starting with this prefix.
    pub fn set_prefix<T: Into<String>>(mut self, v: T) -> Self {
        self.request.prefix = v.into();
        self
    }

    /// Set the full request.
    pub fn with_request<T: Into<model::ListFoosRequest>>(mut self, v: T) -> Self {
        self.request = v.into();
        self
    }

    /// Set the timeout option.
    pub fn with_timeout<T: Into<std::time::Duration>>(mut self, v: T) -> Self {
        let d: std::time::Duration = v.into();
        self.options.insert("timeout".into(), format!("{:?}", d));
        self
    }

    /// Set the user agent option.
    pub fn with_user_agent<T: Into<String>>(mut self, v: T) -> Self {
        self.options.insert("user-agent".into(), v.into());
        self
    }

    pub async fn send(self) -> Result<model::ListFoosResponse> {
        let builder = self
            .client
            .builder(reqwest::Method::GET, "/v0/foos".into())
            .query(&[("alt", "json")])
            .header(
                "x-goog-api-client",
                reqwest::header::HeaderValue::from_static(&info::X_GOOG_API_CLIENT_HEADER),
            );
        let builder = gax::query_parameter::add(builder, "prefix", &self.request.prefix)
            .map_err(Error::other)?;
        let builder = gax::query_parameter::add(builder, "pageToken", &self.request.page_token)
            .map_err(Error::other)?;
        self.client
            .execute(builder, None::<gax::http_client::NoBody>)
            .await
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

pub struct GetFooRequest;

pub struct CreateFooRequest;

pub struct DeleteFooRequest;

pub(crate) mod info {
    const NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    lazy_static::lazy_static! {
        pub(crate) static ref X_GOOG_API_CLIENT_HEADER: String = {
            let ac = gax::api_header::XGoogApiClient{
                name:          NAME,
                version:       VERSION,
                library_type:  gax::api_header::GAPIC,
            };
            ac.header_value()
        };
    }
}
