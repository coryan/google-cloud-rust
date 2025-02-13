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
#![allow(rustdoc::broken_intra_doc_links)]

use crate::Result;
use std::sync::Arc;

/// Implements a client for the Identity and Access Management (IAM) API.
///
/// # Service Description
///
/// An interface for managing Identity and Access Management (IAM) policy
/// bindings.
///
/// # Configuration
///
/// `PolicyBindings` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `PolicyBindings` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `PolicyBindings` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct PolicyBindings {
    inner: Arc<dyn crate::stubs::dynamic::PolicyBindings>,
}

impl PolicyBindings {
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
        T: crate::stubs::PolicyBindings + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn crate::stubs::dynamic::PolicyBindings>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl crate::stubs::PolicyBindings> {
        crate::transport::PolicyBindings::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl crate::stubs::PolicyBindings> {
        Self::build_transport(conf)
            .await
            .map(crate::tracing::PolicyBindings::new)
    }

    /// Creates a policy binding and returns a long-running operation.
    /// Callers will need the IAM permissions on both the policy and target.
    /// Once the binding is created, the policy is applied to the target.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PolicyBinding, model::OperationMetadata>
    /// ) -> Result<model::PolicyBinding> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PolicyBinding, model::OperationMetadata>
    /// ) -> Result<model::PolicyBinding> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [crate::model::PolicyBinding] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::policy_bindings::CreatePolicyBinding::send
    /// [poller()]: crate::builders::policy_bindings::CreatePolicyBinding::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn create_policy_binding(
        &self,
        parent: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::CreatePolicyBinding {
        crate::builders::policy_bindings::CreatePolicyBinding::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Gets a policy binding.
    pub fn get_policy_binding(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::GetPolicyBinding {
        crate::builders::policy_bindings::GetPolicyBinding::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Updates a policy binding and returns a long-running operation.
    /// Callers will need the IAM permissions on the policy and target in the
    /// binding to update, and the IAM permission to remove the existing policy
    /// from the binding. Target is immutable and cannot be updated. Once the
    /// binding is updated, the new policy is applied to the target.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PolicyBinding, model::OperationMetadata>
    /// ) -> Result<model::PolicyBinding> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PolicyBinding, model::OperationMetadata>
    /// ) -> Result<model::PolicyBinding> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [crate::model::PolicyBinding] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::policy_bindings::UpdatePolicyBinding::send
    /// [poller()]: crate::builders::policy_bindings::UpdatePolicyBinding::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn update_policy_binding(
        &self,
        policy_binding: impl Into<crate::model::PolicyBinding>,
    ) -> crate::builders::policy_bindings::UpdatePolicyBinding {
        crate::builders::policy_bindings::UpdatePolicyBinding::new(self.inner.clone())
            .set_policy_binding(policy_binding.into())
    }

    /// Deletes a policy binding and returns a long-running operation.
    /// Callers will need the IAM permissions on both the policy and target.
    /// Once the binding is deleted, the policy no longer applies to the target.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<wkt::Empty, model::OperationMetadata>
    /// ) -> Result<wkt::Empty> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<wkt::Empty, model::OperationMetadata>
    /// ) -> Result<wkt::Empty> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [wkt::Empty] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::policy_bindings::DeletePolicyBinding::send
    /// [poller()]: crate::builders::policy_bindings::DeletePolicyBinding::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn delete_policy_binding(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::DeletePolicyBinding {
        crate::builders::policy_bindings::DeletePolicyBinding::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Lists policy bindings.
    pub fn list_policy_bindings(
        &self,
        parent: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::ListPolicyBindings {
        crate::builders::policy_bindings::ListPolicyBindings::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Search policy bindings by target. Returns all policy binding objects bound
    /// directly to target.
    pub fn search_target_policy_bindings(
        &self,
        parent: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::SearchTargetPolicyBindings {
        crate::builders::policy_bindings::SearchTargetPolicyBindings::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Provides the [Operations][google.longrunning.Operations] service functionality in this service.
    ///
    /// [google.longrunning.Operations]: longrunning::client::Operations
    pub fn get_operation(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::policy_bindings::GetOperation {
        crate::builders::policy_bindings::GetOperation::new(self.inner.clone())
            .set_name(name.into())
    }
}

/// Implements a client for the Identity and Access Management (IAM) API.
///
/// # Service Description
///
/// Manages Identity and Access Management (IAM) principal access boundary
/// policies.
///
/// # Configuration
///
/// `PrincipalAccessBoundaryPolicies` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `PrincipalAccessBoundaryPolicies` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `PrincipalAccessBoundaryPolicies` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct PrincipalAccessBoundaryPolicies {
    inner: Arc<dyn crate::stubs::dynamic::PrincipalAccessBoundaryPolicies>,
}

impl PrincipalAccessBoundaryPolicies {
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
        T: crate::stubs::PrincipalAccessBoundaryPolicies + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn crate::stubs::dynamic::PrincipalAccessBoundaryPolicies>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl crate::stubs::PrincipalAccessBoundaryPolicies> {
        crate::transport::PrincipalAccessBoundaryPolicies::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl crate::stubs::PrincipalAccessBoundaryPolicies> {
        Self::build_transport(conf)
            .await
            .map(crate::tracing::PrincipalAccessBoundaryPolicies::new)
    }

    /// Creates a principal access boundary policy, and returns a long running
    /// operation.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PrincipalAccessBoundaryPolicy, model::OperationMetadata>
    /// ) -> Result<model::PrincipalAccessBoundaryPolicy> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PrincipalAccessBoundaryPolicy, model::OperationMetadata>
    /// ) -> Result<model::PrincipalAccessBoundaryPolicy> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [crate::model::PrincipalAccessBoundaryPolicy] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::principal_access_boundary_policies::CreatePrincipalAccessBoundaryPolicy::send
    /// [poller()]: crate::builders::principal_access_boundary_policies::CreatePrincipalAccessBoundaryPolicy::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn create_principal_access_boundary_policy(
        &self,
        parent: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::CreatePrincipalAccessBoundaryPolicy
    {
        crate::builders::principal_access_boundary_policies::CreatePrincipalAccessBoundaryPolicy::new(self.inner.clone())
            .set_parent ( parent.into() )
    }

    /// Gets a principal access boundary policy.
    pub fn get_principal_access_boundary_policy(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::GetPrincipalAccessBoundaryPolicy {
        crate::builders::principal_access_boundary_policies::GetPrincipalAccessBoundaryPolicy::new(
            self.inner.clone(),
        )
        .set_name(name.into())
    }

    /// Updates a principal access boundary policy.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PrincipalAccessBoundaryPolicy, model::OperationMetadata>
    /// ) -> Result<model::PrincipalAccessBoundaryPolicy> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<model::PrincipalAccessBoundaryPolicy, model::OperationMetadata>
    /// ) -> Result<model::PrincipalAccessBoundaryPolicy> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [crate::model::PrincipalAccessBoundaryPolicy] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::principal_access_boundary_policies::UpdatePrincipalAccessBoundaryPolicy::send
    /// [poller()]: crate::builders::principal_access_boundary_policies::UpdatePrincipalAccessBoundaryPolicy::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn update_principal_access_boundary_policy(
        &self,
        principal_access_boundary_policy: impl Into<crate::model::PrincipalAccessBoundaryPolicy>,
    ) -> crate::builders::principal_access_boundary_policies::UpdatePrincipalAccessBoundaryPolicy
    {
        crate::builders::principal_access_boundary_policies::UpdatePrincipalAccessBoundaryPolicy::new(self.inner.clone())
            .set_principal_access_boundary_policy ( principal_access_boundary_policy.into() )
    }

    /// Deletes a principal access boundary policy.
    ///
    /// # Long running operations
    ///
    /// Calling [poller()] on the resulting builder returns an implementation of
    /// the [lro::Poller] trait. You need to call `Poller::poll` on this
    /// `Poller` at least once to start the LRO. You may periodically poll this
    /// object to find the status of the operation. The poller automatically
    /// extract the final response value and any intermediate metadata values.
    ///
    /// Calling [send()] on the resulting builder starts a LRO (long-Running
    /// Operation). LROs run in the background, and the application may poll
    /// them periodically to find out if they have succeeded, or failed. See
    /// below for instructions on how to manually use the resulting [Operation].
    /// We recommend `poller()` in favor of `send()`.
    ///
    /// ## Polling until completion
    ///
    /// Applications that do not care about intermediate results in a
    /// long-running operation may use the [until_done()] function:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<wkt::Empty, model::OperationMetadata>
    /// ) -> Result<wkt::Empty> {
    ///     poller.until_done().await
    /// }
    /// ```
    ///
    /// This will wait until the LRO completes (successfully or with an error).
    /// Applications can set the [PollingPolicy] and [PollingBackoffPolicy] to
    /// control for how long the function runs.
    ///
    /// ## Polling with detailed metadata updates
    ///
    /// Using the result of [poller()] follows a common pattern:
    ///
    /// ```
    /// # use gax::Result;
    /// # use google_cloud_iam_v3::model;
    /// async fn wait(
    ///     mut poller: impl lro::Poller<wkt::Empty, model::OperationMetadata>
    /// ) -> Result<wkt::Empty> {
    ///     while let Some(p) = poller.poll().await {
    ///         match p {
    ///             lro::PollingResult::Completed(r) => { return r; },
    ///             lro::PollingResult::InProgress(m) => { println!("in progress {m:?}"); },
    ///             lro::PollingResult::PollingError(_) => { /* ignored */ },
    ///         }
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///     }
    ///     Err(gax::error::Error::other("LRO never completed"))
    /// }
    /// ```
    ///
    /// ## Manually polling long-running operations
    ///
    /// If you call [send()], you need to examine the contents of the resulting
    /// [Operation][longrunning::model::Operation] to determine the result of
    /// the operation.
    ///
    /// If the `done` field is `true`, the operation has completed. The `result`
    /// field contains the final response, this will be a [wkt::Empty] (as
    /// an [Any]), or the error (as a `Status`).
    ///
    /// If the `done` field is `false`, the operation has not completed.  The
    /// operation may also include a [crate::model::OperationMetadata] value in the `metadata`
    /// field. This value would also be encoded as an [Any]. The metadata may
    /// include information about how much progress the LRO has made.
    ///
    /// To find out if the operation has completed, use the [get_operation]
    /// method and repeat the steps outlined above.
    ///
    /// Note that most errors on [get_operation] do not indicate that the
    /// long-running operation failed. Long-running operation failures return
    /// the error status in the [result] field.
    ///
    /// [send()]: crate::builders::principal_access_boundary_policies::DeletePrincipalAccessBoundaryPolicy::send
    /// [poller()]: crate::builders::principal_access_boundary_policies::DeletePrincipalAccessBoundaryPolicy::poller
    /// [until_done()]: lro::Poller::until_done
    /// [PollingPolicy]: gax::polling_policy::PollingPolicy
    /// [PollingBackoffPolicy]: gax::polling_backoff_policy::PollingBackoffPolicy
    /// [get_operation]: Self::get_operation
    /// [metadata]: longrunning::model::Operation::result
    /// [name]: longrunning::model::Operation::name
    /// [Operation]: longrunning::model::Operation
    /// [result]: longrunning::model::Operation::result
    /// [Any]: wkt::Any
    pub fn delete_principal_access_boundary_policy(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::DeletePrincipalAccessBoundaryPolicy
    {
        crate::builders::principal_access_boundary_policies::DeletePrincipalAccessBoundaryPolicy::new(self.inner.clone())
            .set_name ( name.into() )
    }

    /// Lists principal access boundary policies.
    pub fn list_principal_access_boundary_policies(
        &self,
        parent: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::ListPrincipalAccessBoundaryPolicies
    {
        crate::builders::principal_access_boundary_policies::ListPrincipalAccessBoundaryPolicies::new(self.inner.clone())
            .set_parent ( parent.into() )
    }

    /// Returns all policy bindings that bind a specific policy if a user has
    /// searchPolicyBindings permission on that policy.
    pub fn search_principal_access_boundary_policy_bindings(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::SearchPrincipalAccessBoundaryPolicyBindings
    {
        crate::builders::principal_access_boundary_policies::SearchPrincipalAccessBoundaryPolicyBindings::new(self.inner.clone())
            .set_name ( name.into() )
    }

    /// Provides the [Operations][google.longrunning.Operations] service functionality in this service.
    ///
    /// [google.longrunning.Operations]: longrunning::client::Operations
    pub fn get_operation(
        &self,
        name: impl Into<std::string::String>,
    ) -> crate::builders::principal_access_boundary_policies::GetOperation {
        crate::builders::principal_access_boundary_policies::GetOperation::new(self.inner.clone())
            .set_name(name.into())
    }
}
