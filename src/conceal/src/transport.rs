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

use crate::Error;

#[derive(Clone, Debug)]
pub(crate) struct FooService {
    client: gax::http_client::ReqwestClient,
}

impl FooService {
    pub async fn new(endpoint: &str) -> crate::Result<Self> {
        let config = gax::http_client::ClientConfig::default()
            .set_credential(auth::Credential::test_credentials());
        let client = gax::http_client::ReqwestClient::new(config, endpoint).await?;
        Ok(Self { client })
    }
}

impl crate::stubs::FooService for FooService {
    async fn list_foos(
        &self,
        request: crate::model::ListFoosRequest,
        _options: crate::RequestOptions,
    ) -> crate::Result<crate::model::ListFoosResponse> {
        let builder = self
            .client
            .builder(reqwest::Method::GET, "/v0/foos".into())
            .query(&[("alt", "json")])
            .header(
                "x-goog-api-client",
                reqwest::header::HeaderValue::from_static(&info::X_GOOG_API_CLIENT_HEADER),
            );
        let builder =
            gax::query_parameter::add(builder, "prefix", &request.prefix).map_err(Error::other)?;
        let builder = gax::query_parameter::add(builder, "pageToken", &request.page_token)
            .map_err(Error::other)?;
        self.client
            .execute(builder, None::<gax::http_client::NoBody>)
            .await
    }

    async fn create_foo(
        &self,
        request: crate::model::CreateFooRequest,
        _options: crate::RequestOptions,
    ) -> crate::Result<crate::model::Foo> {
        let builder = self
            .client
            .builder(reqwest::Method::POST, "/v0/foos".into())
            .query(&[("alt", "json")])
            .header(
                "x-goog-api-client",
                reqwest::header::HeaderValue::from_static(&info::X_GOOG_API_CLIENT_HEADER),
            );
        let builder =
            gax::query_parameter::add(builder, "parent", &request.parent).map_err(Error::other)?;
        let builder =
            gax::query_parameter::add(builder, "foo_id", &request.foo_id).map_err(Error::other)?;
        self.client.execute(builder, Some(request.item)).await
    }
}

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
