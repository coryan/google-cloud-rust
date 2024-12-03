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

use gax::error::Error;
use std::fmt::Debug;

type Result<T> = std::result::Result<T, Error>;

mod gax2 {
    use super::Error;
    use super::Result;
    use auth::Credential;

    #[async_trait::async_trait]
    pub trait Client: Send + Sync {
        async fn get(&self, path: &str) -> Result<reqwest::RequestBuilder>;
        async fn post(&self, path: &str) -> Result<reqwest::RequestBuilder>;
    }

    pub struct Transport {
        http_client: reqwest::Client,
        endpoint: String,
        cred: auth::Credential,
    }

    impl Transport {
        pub fn new(endpoint: String, cred: auth::Credential) -> Self{
            Self { http_client: reqwest::Client::new(), endpoint: endpoint, cred: cred }
        }
    }

    impl std::fmt::Debug for Transport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            write!(f, "Transport[{}]", self.endpoint)
        }
    }

    #[async_trait::async_trait]
    impl Client for Transport {
        async fn get(&self, path: &str) -> Result<reqwest::RequestBuilder> {
            let token = self
                .cred
                .access_token()
                .await
                .map_err(Error::authentication)?;
            Ok(self
                .http_client
                .get(format!("{}/{path}", self.endpoint.as_str()))
                .bearer_auth(token.value))
        }
        async fn post(&self, path: &str) -> Result<reqwest::RequestBuilder> {
            let token = self
                .cred
                .access_token()
                .await
                .map_err(Error::authentication)?;
            Ok(self
                .http_client
                .post(format!("{}/{path}", self.endpoint.as_str()))
                .bearer_auth(token.value))
        }
    }

    #[derive(Debug)]
    struct Tracing<T: Client> {
        inner: T
    }

    impl<T:Client> Tracing<T> {
        fn new(inner: T) -> Self { Self { inner }}
    }

    #[async_trait::async_trait]
    impl<T: Client + std::fmt::Debug> Client for Tracing<T> {
        async fn get(&self, path: &str) -> Result<reqwest::RequestBuilder> {
            println!("{:?}::get({})", &self, &path);
            let r = self.inner.get(path).await?;
            Ok(r)
        }

        async fn post(&self, path: &str) -> Result<reqwest::RequestBuilder> {
            println!("{:?}::post({})", &self, &path);
            let r = self.inner.get(path).await?;
            Ok(r)
        }
    }

    // Using async_trait here is fine, this is an implementation class in the gax
    // library.
    pub async fn execute<I: serde::ser::Serialize, O: serde::de::DeserializeOwned>(
        mut builder: reqwest::RequestBuilder,
        body: Option<I>,
    ) -> super::Result<O> {
        if let Some(body) = body {
            builder = builder.json(&body);
        }
        let resp = builder.send().await.map_err(Error::io)?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let headers = gax::error::convert_headers(resp.headers());
            let body = resp.bytes().await.map_err(Error::io)?;
            return Err(gax::error::HttpError::new(status, headers, Some(body)).into());
        }
        let response = resp.json::<O>().await.map_err(Error::serde)?;
        Ok(response)
    }


    pub(crate) async fn default_credential() -> Result<Credential> {
        let cc = auth::CredentialConfig::builder()
            .scopes(vec![
                "https://www.googleapis.com/auth/cloud-platform".to_string()
            ])
            .build()
            .map_err(Error::authentication)?;
        Credential::find_default(cc)
            .await
            .map_err(Error::authentication)
    }
}

mod client {
    use super::model::*;
    use super::Result;

    pub struct FooService {
        inner: Box<dyn super::gax2::Client>,
    }

    impl FooService {
        pub fn builder() -> super::builder::FooService {
            super::builder::FooService::default()
        }

        pub async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let builder = self
                .inner
                .post(format!("/v1/{}/foos", &req.parent).as_str())
                .await?;
            let builder = builder.query(&[("foo_id", &req.id)]);
            let response = super::gax2::execute(builder, Some(req.body)).await?;
            Ok(response)
        }
        pub async fn get_foo(&self, req: GetFooRequest) -> Result<Foo> {
            let builder = self
                .inner
                .get(format!("/v1/{}", &req.name).as_str())
                .await?;
            let response = super::gax2::execute(builder, None::<()>).await?;
            Ok(response)
        }

        pub(crate) fn new(inner: Box<dyn super::gax2::Client>) -> Self {
            Self { inner }
        }
    }
}

pub mod builder {
    use super::gax2;

    #[derive(Default)]
    pub struct FooService {
        endpoint: Option<String>,
    }

    impl FooService {
        pub fn with_endpoint<T: Into<Option<String>>>(mut self, endpoint: T) -> Self {
            self.endpoint = endpoint.into();
            self
        }
        pub fn with_tracing(mut self) -> Self {
            self
        }

        pub async fn build(self) -> super::Result<super::client::FooService> {
            let endpoint = self.endpoint.unwrap_or_else(|| "https://foo.googleapis.com".to_string());
            let cred = gax2::default_credential().await?;
            let inner = Box::new(gax2::Transport::new(endpoint, cred));
            Ok(super::client::FooService::new(inner))
        }
    }
}

pub mod model {
    #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
    pub struct Foo {
        pub name: String,
        pub things: String,
    }

    #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
    pub struct CreateFooRequest {
        pub parent: String,
        pub id: String,
        pub body: Foo,
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    pub struct GetFooRequest {
        pub name: String,
    }
}
