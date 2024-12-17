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

pub trait FooService: std::fmt::Debug + Send + Sync {
    fn list_foos(&self) -> super::builder::ListFoosRequestBuilder;
    // fn get_foo(&self) -> super::builder::GetFooRequestBuilder;
    // fn create_foo(&self) -> super::builder::CreateFooRequestBuilder;
    // fn delete_foo(&self) -> super::builder::DeleteFooRequestBuilder;
}
