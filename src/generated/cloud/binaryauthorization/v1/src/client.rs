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
#![allow(rustdoc::redundant_explicit_links)]
#![allow(rustdoc::broken_intra_doc_links)]

use crate::Result;
use std::sync::Arc;

/// Implements a client for the Binary Authorization API.
///
/// # Service Description
///
/// Google Cloud Management Service for Binary Authorization admission policies
/// and attestation authorities.
///
/// This API implements a REST model with the following objects:
///
/// * [Policy][google.cloud.binaryauthorization.v1.Policy]
/// * [Attestor][google.cloud.binaryauthorization.v1.Attestor]
///
/// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
/// [google.cloud.binaryauthorization.v1.Policy]: crate::model::Policy
///
/// # Configuration
///
/// `BinauthzManagementServiceV1` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `BinauthzManagementServiceV1` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `BinauthzManagementServiceV1` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct BinauthzManagementServiceV1 {
    inner: Arc<dyn super::stub::dynamic::BinauthzManagementServiceV1>,
}

impl BinauthzManagementServiceV1 {
    /// Creates a new client with the default configuration.
    pub async fn new() -> Result<Self> {
        Self::new_with_config(gax::options::ClientConfig::default()).await
    }

    /// Creates a new client with the specified configuration.
    pub async fn new_with_config(conf: gax::options::ClientConfig) -> Result<Self> {
        let inner = Self::build_inner(conf).await?;
        Ok(Self { inner })
    }

    /// Creates a new client from the provided stub.
    ///
    /// The most common case for calling this function is when mocking the
    /// client.
    pub fn from_stub<T>(stub: T) -> Self
    where
        T: super::stub::BinauthzManagementServiceV1 + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn super::stub::dynamic::BinauthzManagementServiceV1>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::BinauthzManagementServiceV1> {
        super::transport::BinauthzManagementServiceV1::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::BinauthzManagementServiceV1> {
        Self::build_transport(conf)
            .await
            .map(super::tracing::BinauthzManagementServiceV1::new)
    }

    /// A [policy][google.cloud.binaryauthorization.v1.Policy] specifies the [attestors][google.cloud.binaryauthorization.v1.Attestor] that must attest to
    /// a container image, before the project is allowed to deploy that
    /// image. There is at most one policy per project. All image admission
    /// requests are permitted if a project has no policy.
    ///
    /// Gets the [policy][google.cloud.binaryauthorization.v1.Policy] for this project. Returns a default
    /// [policy][google.cloud.binaryauthorization.v1.Policy] if the project does not have one.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    /// [google.cloud.binaryauthorization.v1.Policy]: crate::model::Policy
    pub fn get_policy(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::binauthz_management_service_v_1::GetPolicy {
        super::builder::binauthz_management_service_v_1::GetPolicy::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Creates or updates a project's [policy][google.cloud.binaryauthorization.v1.Policy], and returns a copy of the
    /// new [policy][google.cloud.binaryauthorization.v1.Policy]. A policy is always updated as a whole, to avoid race
    /// conditions with concurrent policy enforcement (or management!)
    /// requests. Returns NOT_FOUND if the project does not exist, INVALID_ARGUMENT
    /// if the request is malformed.
    ///
    /// [google.cloud.binaryauthorization.v1.Policy]: crate::model::Policy
    pub fn update_policy(
        &self,
        policy: impl Into<crate::model::Policy>,
    ) -> super::builder::binauthz_management_service_v_1::UpdatePolicy {
        super::builder::binauthz_management_service_v_1::UpdatePolicy::new(self.inner.clone())
            .set_policy(policy.into())
    }

    /// Creates an [attestor][google.cloud.binaryauthorization.v1.Attestor], and returns a copy of the new
    /// [attestor][google.cloud.binaryauthorization.v1.Attestor]. Returns NOT_FOUND if the project does not exist,
    /// INVALID_ARGUMENT if the request is malformed, ALREADY_EXISTS if the
    /// [attestor][google.cloud.binaryauthorization.v1.Attestor] already exists.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    pub fn create_attestor(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::binauthz_management_service_v_1::CreateAttestor {
        super::builder::binauthz_management_service_v_1::CreateAttestor::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Gets an [attestor][google.cloud.binaryauthorization.v1.Attestor].
    /// Returns NOT_FOUND if the [attestor][google.cloud.binaryauthorization.v1.Attestor] does not exist.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    pub fn get_attestor(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::binauthz_management_service_v_1::GetAttestor {
        super::builder::binauthz_management_service_v_1::GetAttestor::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Updates an [attestor][google.cloud.binaryauthorization.v1.Attestor].
    /// Returns NOT_FOUND if the [attestor][google.cloud.binaryauthorization.v1.Attestor] does not exist.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    pub fn update_attestor(
        &self,
        attestor: impl Into<crate::model::Attestor>,
    ) -> super::builder::binauthz_management_service_v_1::UpdateAttestor {
        super::builder::binauthz_management_service_v_1::UpdateAttestor::new(self.inner.clone())
            .set_attestor(attestor.into())
    }

    /// Lists [attestors][google.cloud.binaryauthorization.v1.Attestor].
    /// Returns INVALID_ARGUMENT if the project does not exist.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    pub fn list_attestors(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::binauthz_management_service_v_1::ListAttestors {
        super::builder::binauthz_management_service_v_1::ListAttestors::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Deletes an [attestor][google.cloud.binaryauthorization.v1.Attestor]. Returns NOT_FOUND if the
    /// [attestor][google.cloud.binaryauthorization.v1.Attestor] does not exist.
    ///
    /// [google.cloud.binaryauthorization.v1.Attestor]: crate::model::Attestor
    pub fn delete_attestor(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::binauthz_management_service_v_1::DeleteAttestor {
        super::builder::binauthz_management_service_v_1::DeleteAttestor::new(self.inner.clone())
            .set_name(name.into())
    }
}

/// Implements a client for the Binary Authorization API.
///
/// # Service Description
///
/// API for working with the system policy.
///
/// # Configuration
///
/// `SystemPolicyV1` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `SystemPolicyV1` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `SystemPolicyV1` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct SystemPolicyV1 {
    inner: Arc<dyn super::stub::dynamic::SystemPolicyV1>,
}

impl SystemPolicyV1 {
    /// Creates a new client with the default configuration.
    pub async fn new() -> Result<Self> {
        Self::new_with_config(gax::options::ClientConfig::default()).await
    }

    /// Creates a new client with the specified configuration.
    pub async fn new_with_config(conf: gax::options::ClientConfig) -> Result<Self> {
        let inner = Self::build_inner(conf).await?;
        Ok(Self { inner })
    }

    /// Creates a new client from the provided stub.
    ///
    /// The most common case for calling this function is when mocking the
    /// client.
    pub fn from_stub<T>(stub: T) -> Self
    where
        T: super::stub::SystemPolicyV1 + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn super::stub::dynamic::SystemPolicyV1>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::SystemPolicyV1> {
        super::transport::SystemPolicyV1::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::SystemPolicyV1> {
        Self::build_transport(conf)
            .await
            .map(super::tracing::SystemPolicyV1::new)
    }

    /// Gets the current system policy in the specified location.
    pub fn get_system_policy(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::system_policy_v_1::GetSystemPolicy {
        super::builder::system_policy_v_1::GetSystemPolicy::new(self.inner.clone())
            .set_name(name.into())
    }
}

/// Implements a client for the Binary Authorization API.
///
/// # Service Description
///
/// BinAuthz Attestor verification
///
/// # Configuration
///
/// `ValidationHelperV1` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `ValidationHelperV1` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `ValidationHelperV1` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct ValidationHelperV1 {
    inner: Arc<dyn super::stub::dynamic::ValidationHelperV1>,
}

impl ValidationHelperV1 {
    /// Creates a new client with the default configuration.
    pub async fn new() -> Result<Self> {
        Self::new_with_config(gax::options::ClientConfig::default()).await
    }

    /// Creates a new client with the specified configuration.
    pub async fn new_with_config(conf: gax::options::ClientConfig) -> Result<Self> {
        let inner = Self::build_inner(conf).await?;
        Ok(Self { inner })
    }

    /// Creates a new client from the provided stub.
    ///
    /// The most common case for calling this function is when mocking the
    /// client.
    pub fn from_stub<T>(stub: T) -> Self
    where
        T: super::stub::ValidationHelperV1 + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn super::stub::dynamic::ValidationHelperV1>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::ValidationHelperV1> {
        super::transport::ValidationHelperV1::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::ValidationHelperV1> {
        Self::build_transport(conf)
            .await
            .map(super::tracing::ValidationHelperV1::new)
    }

    /// Returns whether the given Attestation for the given image URI
    /// was signed by the given Attestor
    pub fn validate_attestation_occurrence(
        &self,
        attestor: impl Into<std::string::String>,
    ) -> super::builder::validation_helper_v_1::ValidateAttestationOccurrence {
        super::builder::validation_helper_v_1::ValidateAttestationOccurrence::new(
            self.inner.clone(),
        )
        .set_attestor(attestor.into())
    }
}
