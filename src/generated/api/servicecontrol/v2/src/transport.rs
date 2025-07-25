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
//
// Code generated by sidekick. DO NOT EDIT.

use crate::Result;
#[allow(unused_imports)]
use gax::error::Error;

/// Implements [ServiceController](super::stub::ServiceController) using a [gaxi::http::ReqwestClient].
#[derive(Clone)]
pub struct ServiceController {
    inner: gaxi::http::ReqwestClient,
}

impl std::fmt::Debug for ServiceController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("ServiceController")
            .field("inner", &self.inner)
            .finish()
    }
}

impl ServiceController {
    pub async fn new(config: gaxi::options::ClientConfig) -> gax::client_builder::Result<Self> {
        let inner = gaxi::http::ReqwestClient::new(config, crate::DEFAULT_HOST).await?;
        Ok(Self { inner })
    }
}

impl super::stub::ServiceController for ServiceController {
    async fn check(
        &self,
        req: crate::model::CheckRequest,
        options: gax::options::RequestOptions,
    ) -> Result<gax::response::Response<crate::model::CheckResponse>> {
        use gax::error::binding::BindingError;
        use gaxi::path_parameter::PathMismatchBuilder;
        use gaxi::path_parameter::try_match;
        use gaxi::routing_parameter::Segment;
        let (builder, method) = None
            .or_else(|| {
                let path = format!(
                    "/v2/services/{}:check",
                    try_match(
                        Some(&req).map(|m| &m.service_name).map(|s| s.as_str()),
                        &[Segment::SingleWildcard]
                    )?,
                );

                let builder = self.inner.builder(reqwest::Method::POST, path);
                let builder = Ok(builder);
                Some(builder.map(|b| (b, reqwest::Method::POST)))
            })
            .ok_or_else(|| {
                let mut paths = Vec::new();
                {
                    let builder = PathMismatchBuilder::default();
                    let builder = builder.maybe_add(
                        Some(&req).map(|m| &m.service_name).map(|s| s.as_str()),
                        &[Segment::SingleWildcard],
                        "service_name",
                        "*",
                    );
                    paths.push(builder.build());
                }
                gax::error::Error::binding(BindingError { paths })
            })??;
        let options = gax::options::internal::set_default_idempotency(
            options,
            gaxi::http::default_idempotency(&method),
        );
        let builder = builder.query(&[("$alt", "json;enum-encoding=int")]).header(
            "x-goog-api-client",
            reqwest::header::HeaderValue::from_static(&crate::info::X_GOOG_API_CLIENT_HEADER),
        );
        self.inner.execute(builder, Some(req), options).await
    }

    async fn report(
        &self,
        req: crate::model::ReportRequest,
        options: gax::options::RequestOptions,
    ) -> Result<gax::response::Response<crate::model::ReportResponse>> {
        use gax::error::binding::BindingError;
        use gaxi::path_parameter::PathMismatchBuilder;
        use gaxi::path_parameter::try_match;
        use gaxi::routing_parameter::Segment;
        let (builder, method) = None
            .or_else(|| {
                let path = format!(
                    "/v2/services/{}:report",
                    try_match(
                        Some(&req).map(|m| &m.service_name).map(|s| s.as_str()),
                        &[Segment::SingleWildcard]
                    )?,
                );

                let builder = self.inner.builder(reqwest::Method::POST, path);
                let builder = Ok(builder);
                Some(builder.map(|b| (b, reqwest::Method::POST)))
            })
            .ok_or_else(|| {
                let mut paths = Vec::new();
                {
                    let builder = PathMismatchBuilder::default();
                    let builder = builder.maybe_add(
                        Some(&req).map(|m| &m.service_name).map(|s| s.as_str()),
                        &[Segment::SingleWildcard],
                        "service_name",
                        "*",
                    );
                    paths.push(builder.build());
                }
                gax::error::Error::binding(BindingError { paths })
            })??;
        let options = gax::options::internal::set_default_idempotency(
            options,
            gaxi::http::default_idempotency(&method),
        );
        let builder = builder.query(&[("$alt", "json;enum-encoding=int")]).header(
            "x-goog-api-client",
            reqwest::header::HeaderValue::from_static(&crate::info::X_GOOG_API_CLIENT_HEADER),
        );
        self.inner.execute(builder, Some(req), options).await
    }
}
