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
    pub trait FooServiceBuilder {
        type Client: super::traits::FooService;

        fn build(self) -> Self::Client;
    }

    #[derive(Debug, Default)]
    pub struct FooService<L> where L: FooServiceBuilder {
        layer: L,
    }

    impl<L> FooService<L> where L: FooServiceBuilder{
        pub fn with_tracing(self) -> FooService<Tracing<L>> {
            FooService::<Tracing<L>> { layer: Tracing::new(self.layer) }
        }

        pub fn build(self) -> L::Client {
            self.layer.build()
        }
    }

    impl FooService<Root> {
        pub const fn new() -> Self {
            Self { layer: Root::new() }
        }
    }

    pub struct Tracing<B> where B: FooServiceBuilder {
        inner: B
    }

    impl<B> Tracing<B> where B: FooServiceBuilder {
        pub fn new(inner: B) -> Self {
            Self {inner}
        }
    }

    impl<B: FooServiceBuilder> FooServiceBuilder for Tracing<B> {
        type Client = super::tracing::FooService<B::Client>;

        fn build(self) -> Self::Client {
            super::tracing::FooService::new(self.inner.build())
        }
    }

    pub struct Root;

    impl Root {
        const fn new() -> Self { Self }
    }

    impl FooServiceBuilder for Root {
        type Client = super::transport::FooService;

        fn build(self) -> Self::Client {
            let client = super::gax3::ReqwestClient::arc();
            Self::Client::new(client)
        }
    }
}

pub mod transport {
    use super::gax3;
    use super::Result;
    use crate::wrapped_execute::model::*;
    use std::sync::Arc;

    pub struct FooService {
        inner: Arc<dyn gax3::Client>, // Arc because eventually may be a stack of things
    }

    impl std::fmt::Debug for FooService {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            write!(f, "SecretManagerService[{:?}]", self.inner)
        }
    }

    impl FooService {
        pub(crate) fn new<T: Into<Arc<dyn gax3::Client>>>(inner: T) -> Self {
            Self {
                inner: inner.into(),
            }
        }
    }

    impl super::traits::FooService for FooService {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            if req.parent == "test-only" {
                // Makes it easy to write tests that do not try to use HTTP connection
                return Ok(Foo {
                    name: format!("{}/foos/{}", &req.parent, &req.id),
                    things: req.body.things,
                });
            }
            let builder = self
                .inner
                .builder(reqwest::Method::POST, format!("/v1/{}", req.parent));
            let response = gax3::execute(
                &self.inner,
                ".foo.FooService.CreateFoo",
                builder,
                Some(req.body),
            )
            .await?;
            Ok(response)
        }
    }

    #[cfg(feature = "unstable-client-trait")]
    #[async_trait::async_trait]
    impl super::dyntraits::FooService for FooService
    {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let request = FooService::create_foo(self, req).await?;
            Ok(request)
        }
    }
}

pub mod client {
    use super::{traits, Result};
    use crate::wrapped_execute::model::*;

    #[derive(Debug)]
    pub struct FooService<T: traits::FooService> {
        inner: T,
    }

    impl<T: traits::FooService> FooService<T> {
        pub(crate) fn new(inner: T) -> Self {
            Self { inner }
        }
    }

    impl<T: traits::FooService> traits::FooService for FooService<T> {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let response = self.inner.create_foo(req).await?;
            Ok(response)      
        }    
    }

    #[cfg(feature = "unstable-client-trait")]
    #[async_trait::async_trait]
    impl<T> super::dyntraits::FooService for FooService<T>
    where
        T: traits::FooService,
    {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let request = FooService::create_foo(self, req).await?;
            Ok(request)
        }
    }
}

pub mod tracing {
    use super::{traits, Result};
    use crate::wrapped_execute::model::*;

    #[derive(Debug)]
    pub struct FooService<T: traits::FooService> {
        inner: T,
    }

    impl<T: traits::FooService> FooService<T> {
        pub(crate) fn new(inner: T) -> Self {
            Self { inner }
        }
    }

    impl<T: traits::FooService> traits::FooService for FooService<T> {
        #[tracing::instrument(ret)]
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let response = self.inner.create_foo(req).await?;
            Ok(response)      
        }    
    }

    #[cfg(feature = "unstable-client-trait")]
    #[async_trait::async_trait]
    impl<T> super::dyntraits::FooService for FooService<T>
    where
        T: traits::FooService,
    {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let request = FooService::create_foo(self, req).await?;
            Ok(request)
        }
    }
}


pub mod traits {
    use super::Result;
    use crate::wrapped_execute::model::*;

    pub trait FooService: std::fmt::Debug + Send + Sync {
        fn create_foo(&self, req: CreateFooRequest) -> impl std::future::Future<Output = Result<Foo>> + Send;
    }
}

#[cfg(feature = "unstable-client-trait")]
pub mod dyntraits {
    use super::Result;
    use super::model::{CreateFooRequest, Foo};

    #[async_trait::async_trait]
    pub trait FooService: Send + Sync {
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
    pub trait Client: std::fmt::Debug + Send + Sync {
        fn builder(&self, method: reqwest::Method, path: String) -> reqwest::RequestBuilder;
        async fn execute(
            &self,
            rpc_name: &str,
            builder: reqwest::RequestBuilder,
        ) -> Result<reqwest::Response>;
    }

    pub struct ReqwestClient {
        inner: reqwest::Client,
        endpoint: String,
    }

    impl std::fmt::Debug for ReqwestClient {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            write!(f, "Client[{}]", self.endpoint)
        }
    }
    

    impl ReqwestClient {
        pub fn default() -> Self {
            Self {
                inner: reqwest::Client::new(),
                endpoint: "https://foo.googleapis.com".to_string(),
            }
        }
        pub fn arc() -> std::sync::Arc<dyn Client> {
            std::sync::Arc::new(Self::default())
        }
    }

    #[async_trait::async_trait]
    impl Client for ReqwestClient {
        fn builder(&self, method: reqwest::Method, path: String) -> reqwest::RequestBuilder {
            self.inner
                .request(method, format!("{}/{path}", self.endpoint))
        }

        async fn execute(
            &self,
            _rpc_name: &str,
            builder: reqwest::RequestBuilder,
        ) -> Result<reqwest::Response> {
            builder.send().await.map_err(Error::io)
        }
    }
}
