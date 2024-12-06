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
    use super::*;
    use crate::gax5;

    #[derive(Clone, Debug, Default)]
    pub struct FooService {
        tracing: bool,
        // This serves as a proxy for all the configuration optiosn, retry loops, timeouts, etc.
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

        pub fn build(self) -> client::FooService {
            return client::FooService::new(self.build_inner());
        }

        fn build_inner(self) -> Box<dyn traits::dynamic::FooService> {
            if Self::tracing_enabled(self.tracing) {
                let client = StaticBuilder::new(self.endpoint).build_with_tracing();
                return Box::new(client);
            }
            let client = StaticBuilder::new(self.endpoint).build();
            return Box::new(client);
        }

        fn tracing_enabled(tracing: bool) -> bool {
            if tracing {
                return true;
            }
            return std::env::var("GOOGLE_CLOUD_RUST_TRACING")
                .map(|v| v == "true")
                .unwrap_or(false);
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct StaticBuilder {
        endpoint: Option<String>,
    }

    impl StaticBuilder {
        pub fn new<T: Into<Option<String>>>(endpoint: T) -> Self {
            Self {
                endpoint: endpoint.into(),
            }
        }

        pub fn build_with_tracing(self) -> impl traits::FooService {
            let client = self.build();
            return tracing::FooService::new(client);
        }

        pub fn build(self) -> impl traits::FooService {
            let ep = self.endpoint.unwrap_or(ENDPOINT.into());
            let client = gax5::ReqwestClient::new(ep);
            return transport::FooService::new(client);
        }
    }
}

pub mod client {
    use std::fmt::Pointer;

    use super::{traits, Result};
    use crate::wrapped_execute::model::*;

    pub struct FooService {
        inner: Box<dyn traits::dynamic::FooService>,
    }

    impl FooService {
        pub(crate) fn new(inner: Box<dyn traits::dynamic::FooService>) -> Self {
            Self { inner }
        }
    }

    impl std::fmt::Debug for FooService {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.inner.fmt(f)
        }
    }

    impl traits::FooService for FooService {
        async fn create_foo(&self, req: CreateFooRequest) -> Result<Foo> {
            let response = self.inner.create_foo(req).await?;
            Ok(response)
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
    use crate::wrapped_execute::model;

    pub trait FooService: std::fmt::Debug + Send + Sync {
        fn create_foo(
            &self,
            req: model::CreateFooRequest,
        ) -> impl std::future::Future<Output = Result<model::Foo>> + Send;
    }

    pub(crate) mod dynamic {
        use super::model;
        use super::Result;

        #[async_trait::async_trait]
        pub trait FooService: Send + Sync {
            async fn create_foo(&self, req: model::CreateFooRequest) -> Result<model::Foo>;
        }

        #[async_trait::async_trait]
        impl<T: super::FooService> FooService for T {
            async fn create_foo(&self, req: model::CreateFooRequest) -> Result<model::Foo> {
                let request = T::create_foo(self, req).await?;
                Ok(request)
            }
        }
    }
}

#[cfg(feature = "unstable-dyntraits")]
pub mod dyntraits {
    pub use super::traits::dynamic::FooService;
}
