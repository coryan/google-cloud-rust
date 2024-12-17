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

/// The internal trait for `FooService`.
pub trait FooService: std::fmt::Debug + Send + Sync {
    fn list_foos(
        &self,
        request: crate::model::ListFoosRequest,
        options: crate::RequestOptions,
    ) -> impl std::future::Future<Output = crate::Result<crate::model::ListFoosResponse>> + Send;

    fn create_foo(
        &self,
        request: crate::model::CreateFooRequest,
        options: crate::RequestOptions,
    ) -> impl std::future::Future<Output = crate::Result<crate::model::Foo>> + Send;
    // fn get_foo(&self) -> super::builder::GetFooRequestBuilder;
    // fn delete_foo(&self) -> super::builder::DeleteFooRequestBuilder;
}

pub(crate) mod dyncompatible {
    #[async_trait::async_trait]
    pub(crate) trait FooService: std::fmt::Debug + Send + Sync {
        async fn list_foos(
            &self,
            request: crate::model::ListFoosRequest,
            options: crate::RequestOptions,
        ) -> crate::Result<crate::model::ListFoosResponse>;

        async fn create_foo(
            &self,
            request: crate::model::CreateFooRequest,
            options: crate::RequestOptions,
        ) -> crate::Result<crate::model::Foo>;
        // fn get_foo(&self) -> super::builder::GetFooRequestBuilder;
        // fn delete_foo(&self) -> super::builder::DeleteFooRequestBuilder;
    }

    #[async_trait::async_trait]
    impl<T> FooService for T where T: super::FooService + Send + Sync {
        async fn list_foos(
            &self,
            request: crate::model::ListFoosRequest,
            options: crate::RequestOptions,
        ) -> crate::Result<crate::model::ListFoosResponse> {
            T::list_foos(&self, request, options).await
        }

        async fn create_foo(
            &self,
            request: crate::model::CreateFooRequest,
            options: crate::RequestOptions,
        ) -> crate::Result<crate::model::Foo> {
            T::create_foo(&self, request, options).await
        }
    }
}
