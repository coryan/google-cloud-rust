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

/// The error type for all operations in this crate.
pub use gax::error::Error;

/// The result type for all operations in this crate.
pub type Result<T> = std::result::Result<T, gax::error::Error>;

/// A placeholder for request options.
pub type RequestOptions = std::collections::HashMap<String, String>;

pub mod builder;
pub mod client;
pub mod model;
pub mod traits;

pub mod stubs;
pub(crate) mod transport;

pub mod server;
