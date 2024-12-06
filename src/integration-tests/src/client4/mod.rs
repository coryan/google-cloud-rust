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
    use crate::gax4;

    pub trait FooServiceBuilder {
        type Client: super::traits::FooService;

        fn build(self) -> Self::Client;
    }

    #[derive(Debug, Default)]
    pub struct FooService<L>
    where
        L: FooServiceBuilder,
    {
        layer: L,
    }

    impl<L> FooService<L>
    where
        L: FooServiceBuilder,
    {
        pub fn with_tracing(self) -> FooService<Tracing<L>> {
            FooService::<Tracing<L>> {
                layer: Tracing::new(self.layer),
            }
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

    pub struct Tracing<B>
    where
        B: FooServiceBuilder,
    {
        inner: B,
    }

    impl<B> Tracing<B>
    where
        B: FooServiceBuilder,
    {
        pub fn new(inner: B) -> Self {
            Self { inner }
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
        const fn new() -> Self {
            Self
        }
    }

    impl FooServiceBuilder for Root {
        type Client = super::transport::FooService<gax4::ReqwestClient>;

        fn build(self) -> Self::Client {
            let client = gax4::ReqwestClient::new("http://foo.googleapis.com".into());
            Self::Client::new(client)
        }
    }
}

pub mod transport {
    use super::Result;
    use crate::gax4;
    use crate::wrapped_execute::model::*;

    pub struct FooService<T>
    where
        T: gax4::HttpClient,
    {
        http_client: T,
    }

    impl<T: gax4::HttpClient> std::fmt::Debug for FooService<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            f.debug_struct("FooService")
                .field("http_client", &self.http_client)
                .finish()
        }
    }

    impl<T: gax4::HttpClient> FooService<T> {
        pub(crate) fn new(http_client: T) -> Self {
            Self { http_client }
        }
    }

    impl<T: gax4::HttpClient> super::traits::FooService for FooService<T> {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            if req.parent == "test-only" {
                // Makes it easy to write tests that do not try to use HTTP connection
                return Ok(Foo {
                    name: format!("{}/foos/{}", &req.parent, &req.id),
                    things: req.body.things,
                });
            }
            let builder = http::request::Builder::new().method(http::Method::POST);
            let response = gax4::execute(
                &self.http_client,
                builder,
                format!("/v1/{}", req.parent),
                Some(req.body),
            )
            .await?;
            Ok(response)
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
}

pub mod traits {
    use super::Result;
    use crate::wrapped_execute::model::*;

    pub trait FooService: std::fmt::Debug + Send + Sync {
        fn create_foo(
            &self,
            req: CreateFooRequest,
        ) -> impl std::future::Future<Output = Result<Foo>> + Send;
    }
}

#[cfg(feature = "unstable-dyntraits")]
pub mod dyntraits {
    use super::model::{CreateFooRequest, Foo};
    use super::Result;

    #[async_trait::async_trait]
    pub trait FooService: Send + Sync {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo>;
    }

    #[async_trait::async_trait]
    impl<T: super::traits::FooService> FooService for T {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let request = T::create_foo(self, req).await?;
            Ok(request)
        }
    }
}
