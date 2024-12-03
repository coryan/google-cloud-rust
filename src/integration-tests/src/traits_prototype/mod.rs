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

#[allow(async_fn_in_trait)]
pub trait Service: Debug + Send + Sync {
    async fn rpc(&self, req: model::Request) -> Result<model::Response, Error>;
}

pub struct Builder {
}

impl Builder {
    pub fn build() -> impl Service {
        Transport{}
    }
    pub fn with_tracing<T>(svc: T) -> impl Service 
    where T: Service + Debug {
        Tracing::new(svc)
    }
    pub fn with_retry<T>(count: u32, svc: T) -> impl Service 
    where T: Service + Debug {
        Retry::new(count, svc)
    }
}

struct Transport {}

impl Service for Transport {
    async fn rpc(&self, req: model::Request) -> Result<model::Response, Error> {
        Ok(model::Response{ name: format!("{}/foos/{}", req.parent, req.id)})
    }
}

impl Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Transport[]")
    }
}

struct Tracing<T> where T: Service + Debug {
    inner: T,
}

impl<T: Service + Debug> Tracing<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    } 
}

impl<T> Service for Tracing<T> where T: Service + Debug {
    async fn rpc(&self, req: model::Request) -> Result<model::Response, Error> {
        self.inner.rpc(req).await
    }
}

impl<T> Debug for Tracing<T> where T: Service + Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Tracing<{:?}>", &self.inner)
    }
}

struct Retry<T> where T: Service + Debug {
    count: u32,
    inner: T,
}

impl<T: Service + Debug> Retry<T> {
    pub fn new(count: u32, inner: T) -> Self {
        Self { count, inner }
    } 
}

impl<T> Service for Retry<T> where T: Service + Debug {
    async fn rpc(&self, req: model::Request) -> Result<model::Response, Error> {
        for _ in 0..self.count {
            let r =         self.inner.rpc(req.clone()).await;
            if r.is_ok() {
                return r;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        Err(Error::other("too many failures"))
    }
}

impl<T> Debug for Retry<T> where T: Service + Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Retry<{}, {:?}>", self.count, &self.inner)
    }
}

pub mod model {
    #[derive(Clone)]
    pub struct Request {
        pub parent: String,
        pub id: String,
    }

    #[derive(Clone)]
    pub struct Response {
        pub name: String,
    }
} 
