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

use crate::{client_options::ClientOptions, tokens::AccessTokenSource};
use gax::error::{Error, HttpError};

pub const ENDPOINT: &str = "https://secretmanager.googleapis.com";

pub struct DefaultSecretManagerService {
    client: reqwest::Client,
    endpoint: String,
    credentials: Box<dyn crate::tokens::AccessTokenSource + Send + Sync>,
}

impl DefaultSecretManagerService {
    pub async fn default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from(ClientOptions::default()).await
    }

    pub async fn from(opts: ClientOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint = opts.endpoint.unwrap_or(ENDPOINT.into());
        let client = opts.client_builder.build()?;
        let credentials = Self::default_credentials(opts.credentials).await?;
        Ok(Self {
            client,
            endpoint,
            credentials,
        })
    }

    async fn default_credentials(
        configured: Option<Box<dyn AccessTokenSource + Send + Sync>>,
    ) -> Result<Box<dyn AccessTokenSource + Send + Sync>, Box<dyn std::error::Error>> {
        if let Some(c) = configured {
            return Ok(c);
        }
        Ok(crate::tokens::default_credentials().await?)
    }

    async fn send_impl<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let token = self.credentials.token().await?;
        let response = request.bearer_auth(token.value).send().await.map_err(Error::io)?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let headers = gax::error::convert_headers(response.headers());
            let body = response.bytes().await.map_err(Error::io)?;
            return Err(HttpError::new(status, headers, Some(body)).into());
        }
        let response = response
            .json::<T>()
            .await
            .map_err(Error::serde)?;
        Ok(response)
    }
}

#[async_trait::async_trait]
impl crate::SecretManagerService for DefaultSecretManagerService {
    /// Lists [Secrets][google.cloud.secretmanager.v1.Secret].
    async fn list_secrets(
        &self,
        req: crate::model::ListSecretsRequest,
    ) -> Result<crate::model::ListSecretsResponse, Box<dyn std::error::Error>> {
        let builder = self.client
            .get(format!("{}/v1/{}/secrets", self.endpoint, req.parent,))
            .query(&[("alt", "json")]);
        let builder =
            gax::query_parameter::add(builder, "pageSize", &req.page_size).map_err(Error::other)?;
        let builder = gax::query_parameter::add(builder, "pageToken", &req.page_token)
            .map_err(Error::other)?;
        let builder =
            gax::query_parameter::add(builder, "filter", &req.filter).map_err(Error::other)?;
        let response = self.send_impl::<crate::model::ListSecretsResponse>(builder).await?;
        Ok(response)
    }

    /// Creates a new [Secret][google.cloud.secretmanager.v1.Secret] containing no
    /// [SecretVersions][google.cloud.secretmanager.v1.SecretVersion].
    async fn create_secret(
        &self,
        req: crate::model::CreateSecretRequest,
    ) -> Result<crate::model::Secret, Box<dyn std::error::Error>> {
        let builder = self.client
            .post(format!("{}/v1/{}/secrets", self.endpoint, req.parent,))
            .json(&req.secret)
            .query(&[("alt", "json")]);
        let builder =
            gax::query_parameter::add(builder, "secretId", &req.secret_id).map_err(Error::other)?;
        let response = self.send_impl::<crate::model::Secret>(builder).await?;
        Ok(response)
    }

    /// Gets metadata for a given [Secret][google.cloud.secretmanager.v1.Secret].
    async fn get_secret(
        &self,
        req: crate::model::GetSecretRequest,
    ) -> Result<crate::model::Secret, Box<dyn std::error::Error>> {
        let builder = self.client
            .get(format!("{}/v1/{}", self.endpoint, req.name,))
            .query(&[("alt", "json")]);
        let response = self.send_impl::<crate::model::Secret>(builder).await?;
        Ok(response)
    }

    /// Deletes a [Secret][google.cloud.secretmanager.v1.Secret].
    async fn delete_secret(
        &self,
        req: crate::model::DeleteSecretRequest,
    ) -> Result<wkt::Empty, Box<dyn std::error::Error>> {
        let builder = self.client
            .delete(format!("{}/v1/{}", self.endpoint, req.name,))
            .query(&[("alt", "json")]);
        let response = self.send_impl::<wkt::Empty>(builder).await?;
        Ok(response)
    }
}
