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
use crate::wrapped_execute::model::*;

type Result<T> = std::result::Result<T, Error>;

mod gax3 {
    use super::*;
    
    type Result<T> = std::result::Result<T, Error>;
    trait Client {
        async fn send(request: http::Request<reqwest::Body>) -> Result<reqwest::Response>;
    }



}
