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
type Result<T> = std::result::Result<T, Error>;

pub mod model {
    pub use crate::wrapped_execute::model::*;
}

pub mod builder {

#[derive(Debug, Default)]
pub struct FooService;

impl FooService {
    pub fn build(self) -> super::transport::FooService {
        super::transport::FooService::new(super::gax3::ReqwestClient::arc() )
    }
    // TODO with_retry()
    // TODO with_tracing()
}
}

pub mod transport {
    use super::builder;
    use crate::wrapped_execute::model::*;
    use super::gax3;
    use super::Result;
    use std::sync::Arc;

    pub struct FooService {
        inner: Arc<dyn gax3::Client>,
    }

    impl FooService {
        pub fn builder() -> builder::FooService {
            builder::FooService::default()
        }
        pub async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let builder = self.inner.builder(reqwest::Method::POST, format!("/v1/{}", req.parent));
            let response = gax3::execute(&self.inner, ".foo.FooService.CreateFoo", builder, Some(req.body)).await?;
            Ok(response)
        }

        pub(crate) fn new<T: Into<Arc<dyn gax3::Client>>>(inner: T) -> Self {
            Self { inner: inner.into() }
        }
    }

    #[cfg(feature = "unstable-client-trait")]
    #[async_trait::async_trait]
    impl super::client::FooService for FooService {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let request= FooService::create_foo(self, req).await?;
            Ok(request)
        }
    }
}

#[cfg(feature = "unstable-client-trait")]
pub mod client {
    use crate::wrapped_execute::model::*;
    use super::Result;

    #[async_trait::async_trait]
    pub trait FooService {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo>;
    }
}

mod gax3 {
    use super::*;

    pub async fn execute<I: serde::ser::Serialize, O: serde::de::DeserializeOwned>(
        client: &std::sync::Arc<dyn Client>,
        rpc_name: &str,
        mut builder: reqwest::RequestBuilder,
        body: Option<I>,
    ) -> super::Result<O> {
        if let Some(body) = body {
            builder = builder.json(&body);
        }
        let response = client.execute(rpc_name, builder).await?;
        // More error handling needed here....
        let response = response.json::<O>().await.map_err(Error::serde)?;
        Ok(response)
    }
    
    #[async_trait::async_trait]
    pub trait Client: Send + Sync {
        fn builder(&self, method: reqwest::Method, path: String) -> reqwest::RequestBuilder;
        async fn execute(&self, rpc_name: &str, builder: reqwest::RequestBuilder) -> Result<reqwest::Response>;
    }

    pub struct ReqwestClient {
        inner: reqwest::Client,
        endpoint: String,
    }

    impl ReqwestClient {
        pub fn default() -> Self { Self { inner: reqwest::Client::new(), endpoint: "https://foo.googleapis.com".to_string() }}
        pub fn arc() -> std::sync::Arc<dyn Client> {
            std::sync::Arc::new(Self::default())
        }
    }

    #[async_trait::async_trait]
    impl Client for ReqwestClient {
        fn builder(&self, method: reqwest::Method, path: String) -> reqwest::RequestBuilder {
            self.inner.request(method, format!("{}/{path}", self.endpoint))
        }

        async fn execute(&self, _rpc_name: &str, builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
            builder.send().await.map_err(Error::io)
        }
    }

}
