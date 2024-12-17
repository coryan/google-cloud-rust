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

use gax::http_client::ClientConfig;

#[derive(Clone, Debug)]
pub struct FooService {
    client: gax::http_client::ReqwestClient,
}

impl FooService {
    pub async fn new(endpoint: &str) -> crate::Result<Self> {
        let config = ClientConfig::default().set_credential(auth::Credential::test_credentials());
        let client = gax::http_client::ReqwestClient::new(config, endpoint).await?;
        Ok(Self { client })
    }
}

impl super::traits::FooService for FooService {
    fn list_foos(&self) -> super::builder::ListFoosRequest {
        super::builder::ListFoosRequest::new(self.client.clone())
    }

    fn create_foo(&self) -> super::builder::CreateFooRequest {
        super::builder::CreateFooRequest::new(self.client.clone())
    }
}
