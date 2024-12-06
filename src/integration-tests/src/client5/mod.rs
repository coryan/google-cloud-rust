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

const ENDPOINT: &str = "https://foo.googleapis.com";

pub mod builder {
    use crate::gax5;
    use super::*;

    #[derive(Clone, Debug, Default)]
    pub struct FooService {
        tracing: bool,
        endpoint: Option<String>,
    }

    impl FooService {
        pub fn with_endpoint<T: Into<String>>(mut self, value: T) -> Self {
            self.endpoint = Some(value.into());
            self
        }

        pub fn with_tracing(mut self) -> Self {
            self.tracing = true;
            self
        }

        pub fn build(self) -> Box<dyn dyntraits::FooService> {
            let ep = self.endpoint.unwrap_or(ENDPOINT.into());
            let client = gax5::ReqwestClient::new(ep);
            let client = transport::FooService::new(client);

            if Self::tracing_enabled(self.tracing) {
                return Box::new(tracing::FooService::new(client));
            }
            return Box::new(client);
        }

        fn tracing_enabled(tracing: bool) -> bool {
            if tracing {
                return true;
            }
            return std::env::var("GOOGLE_CLOUD_RUST_TRACING").map(|v| v == "true").unwrap_or(false);
        }

    }
}

pub mod transport {
    use super::Result;
    use crate::gax5;
    use crate::wrapped_execute::model::*;

    pub struct FooService<T>
    where
        T: gax5::HttpClient,
    {
        http_client: T,
    }

    impl<T: gax5::HttpClient> std::fmt::Debug for FooService<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
            f.debug_struct("FooService")
                .field("http_client", &self.http_client)
                .finish()
        }
    }

    impl<T: gax5::HttpClient> FooService<T> {
        pub(crate) fn new(http_client: T) -> Self {
            Self { http_client }
        }
    }

    impl<T: gax5::HttpClient> super::traits::FooService for FooService<T> {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            if req.parent == "test-only" {
                // Makes it easy to write tests that do not try to use HTTP connection
                return Ok(Foo {
                    name: format!("{}/foos/{}", &req.parent, &req.id),
                    things: req.body.things,
                });
            }
            let builder = http::request::Builder::new().method(http::Method::POST);
            let response = gax5::execute(
                &self.http_client,
                builder,
                format!("/v1/{}", req.parent),
                Some(req.body),
                gax5::Config::default(),
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

#[cfg(feature = "unstable-client-trait")]
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
