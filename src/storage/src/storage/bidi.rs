// Copyright 2025 Google LLC
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

use super::request_options::RequestOptions;
use crate::Error;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bidi<S = BidiTransport>
where
    S: BidiStub + 'static,
{
    stub: std::sync::Arc<S>,
    options: RequestOptions,
}

impl<S> Bidi<S> {
    pub(crate) fn new(builder: super::client::ClientBuilder) -> gax::client_builder::Result<Self> {
        use gax::client_builder::Error;
        let client = gaxi::grpc::Client::new(builder.config, DEFAULT_HOST);
        Ok(Self { stub, options })
    }
}

impl crate::storage::client::ClientBuilder {
    pub fn build_bidi(self) -> gax::client_builder::Result<Bidi> {
        Bidi::new(self)
    }
}

trait BidiStub: std::fmt::Debug + Send + Sync {}

#[derive(Debug)]
struct BidiTransport {}

impl BidiStub for BidiTransport {}
