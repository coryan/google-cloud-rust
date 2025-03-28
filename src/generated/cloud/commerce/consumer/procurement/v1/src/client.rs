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

/// Implements a client for the Cloud Commerce Consumer Procurement API.
///
/// # Service Description
///
/// Service for managing licenses.
///
/// # Configuration
///
/// `LicenseManagementService` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `LicenseManagementService` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `LicenseManagementService` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct LicenseManagementService {
    inner: Arc<dyn super::stub::dynamic::LicenseManagementService>,
}

impl LicenseManagementService {
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
        T: super::stub::LicenseManagementService + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn super::stub::dynamic::LicenseManagementService>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::LicenseManagementService> {
        super::transport::LicenseManagementService::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::LicenseManagementService> {
        Self::build_transport(conf)
            .await
            .map(super::tracing::LicenseManagementService::new)
    }

    /// Gets the license pool.
    pub fn get_license_pool(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::license_management_service::GetLicensePool {
        super::builder::license_management_service::GetLicensePool::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Updates the license pool if one exists for this Order.
    pub fn update_license_pool(
        &self,
        license_pool: impl Into<crate::model::LicensePool>,
    ) -> super::builder::license_management_service::UpdateLicensePool {
        super::builder::license_management_service::UpdateLicensePool::new(self.inner.clone())
            .set_license_pool(license_pool.into())
    }

    /// Assigns a license to a user.
    pub fn assign(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::license_management_service::Assign {
        super::builder::license_management_service::Assign::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Unassigns a license from a user.
    pub fn unassign(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::license_management_service::Unassign {
        super::builder::license_management_service::Unassign::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Enumerates all users assigned a license.
    pub fn enumerate_licensed_users(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::license_management_service::EnumerateLicensedUsers {
        super::builder::license_management_service::EnumerateLicensedUsers::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Provides the [Operations][google.longrunning.Operations] service functionality in this service.
    ///
    /// [google.longrunning.Operations]: longrunning::client::Operations
    pub fn get_operation(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::license_management_service::GetOperation {
        super::builder::license_management_service::GetOperation::new(self.inner.clone())
            .set_name(name.into())
    }
}

/// Implements a client for the Cloud Commerce Consumer Procurement API.
///
/// # Service Description
///
/// ConsumerProcurementService allows customers to make purchases of products
/// served by the Cloud Commerce platform.
///
/// When purchases are made, the
/// [ConsumerProcurementService][google.cloud.commerce.consumer.procurement.v1.ConsumerProcurementService]
/// programs the appropriate backends, including both Google's own
/// infrastructure, as well as third-party systems, and to enable billing setup
/// for charging for the procured item.
///
/// [google.cloud.commerce.consumer.procurement.v1.ConsumerProcurementService]: crate::client::ConsumerProcurementService
///
/// # Configuration
///
/// `ConsumerProcurementService` has various configuration parameters, the defaults should
/// work with most applications.
///
/// # Pooling and Cloning
///
/// `ConsumerProcurementService` holds a connection pool internally, it is advised to
/// create one and the reuse it.  You do not need to wrap `ConsumerProcurementService` in
/// an [Rc](std::rc::Rc) or [Arc] to reuse it, because it already uses an `Arc`
/// internally.
#[derive(Clone, Debug)]
pub struct ConsumerProcurementService {
    inner: Arc<dyn super::stub::dynamic::ConsumerProcurementService>,
}

impl ConsumerProcurementService {
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
        T: super::stub::ConsumerProcurementService + 'static,
    {
        Self {
            inner: Arc::new(stub),
        }
    }

    async fn build_inner(
        conf: gax::options::ClientConfig,
    ) -> Result<Arc<dyn super::stub::dynamic::ConsumerProcurementService>> {
        if conf.tracing_enabled() {
            return Ok(Arc::new(Self::build_with_tracing(conf).await?));
        }
        Ok(Arc::new(Self::build_transport(conf).await?))
    }

    async fn build_transport(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::ConsumerProcurementService> {
        super::transport::ConsumerProcurementService::new(conf).await
    }

    async fn build_with_tracing(
        conf: gax::options::ClientConfig,
    ) -> Result<impl super::stub::ConsumerProcurementService> {
        Self::build_transport(conf)
            .await
            .map(super::tracing::ConsumerProcurementService::new)
    }

    /// Creates a new [Order][google.cloud.commerce.consumer.procurement.v1.Order].
    ///
    /// This API only supports GCP spend-based committed use
    /// discounts specified by GCP documentation.
    ///
    /// The returned long-running operation is in-progress until the backend
    /// completes the creation of the resource. Once completed, the order is
    /// in
    /// [OrderState.ORDER_STATE_ACTIVE][google.cloud.commerce.consumer.procurement.v1.OrderState.ORDER_STATE_ACTIVE].
    /// In case of failure, the order resource will be removed.
    ///
    /// [google.cloud.commerce.consumer.procurement.v1.Order]: crate::model::Order
    ///
    /// # Long running operations
    ///
    /// This method is used to start, and/or poll a [long-running Operation].
    /// The [Working with long-running operations] chapter in the [user guide]
    /// covers these operations in detail.
    ///
    /// [long-running operation]: https://google.aip.dev/151
    /// [user guide]: https://googleapis.github.io/google-cloud-rust/
    /// [working with long-running operations]: https://googleapis.github.io/google-cloud-rust/working_with_long_running_operations.html
    pub fn place_order(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::PlaceOrder {
        super::builder::consumer_procurement_service::PlaceOrder::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Returns the requested
    /// [Order][google.cloud.commerce.consumer.procurement.v1.Order] resource.
    ///
    /// [google.cloud.commerce.consumer.procurement.v1.Order]: crate::model::Order
    pub fn get_order(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::GetOrder {
        super::builder::consumer_procurement_service::GetOrder::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Lists [Order][google.cloud.commerce.consumer.procurement.v1.Order]
    /// resources that the user has access to, within the scope of the parent
    /// resource.
    ///
    /// [google.cloud.commerce.consumer.procurement.v1.Order]: crate::model::Order
    pub fn list_orders(
        &self,
        parent: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::ListOrders {
        super::builder::consumer_procurement_service::ListOrders::new(self.inner.clone())
            .set_parent(parent.into())
    }

    /// Modifies an existing
    /// [Order][google.cloud.commerce.consumer.procurement.v1.Order] resource.
    ///
    /// [google.cloud.commerce.consumer.procurement.v1.Order]: crate::model::Order
    ///
    /// # Long running operations
    ///
    /// This method is used to start, and/or poll a [long-running Operation].
    /// The [Working with long-running operations] chapter in the [user guide]
    /// covers these operations in detail.
    ///
    /// [long-running operation]: https://google.aip.dev/151
    /// [user guide]: https://googleapis.github.io/google-cloud-rust/
    /// [working with long-running operations]: https://googleapis.github.io/google-cloud-rust/working_with_long_running_operations.html
    pub fn modify_order(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::ModifyOrder {
        super::builder::consumer_procurement_service::ModifyOrder::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Cancels an existing
    /// [Order][google.cloud.commerce.consumer.procurement.v1.Order]. Every product
    /// procured in the Order will be cancelled.
    ///
    /// [google.cloud.commerce.consumer.procurement.v1.Order]: crate::model::Order
    ///
    /// # Long running operations
    ///
    /// This method is used to start, and/or poll a [long-running Operation].
    /// The [Working with long-running operations] chapter in the [user guide]
    /// covers these operations in detail.
    ///
    /// [long-running operation]: https://google.aip.dev/151
    /// [user guide]: https://googleapis.github.io/google-cloud-rust/
    /// [working with long-running operations]: https://googleapis.github.io/google-cloud-rust/working_with_long_running_operations.html
    pub fn cancel_order(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::CancelOrder {
        super::builder::consumer_procurement_service::CancelOrder::new(self.inner.clone())
            .set_name(name.into())
    }

    /// Provides the [Operations][google.longrunning.Operations] service functionality in this service.
    ///
    /// [google.longrunning.Operations]: longrunning::client::Operations
    pub fn get_operation(
        &self,
        name: impl Into<std::string::String>,
    ) -> super::builder::consumer_procurement_service::GetOperation {
        super::builder::consumer_procurement_service::GetOperation::new(self.inner.clone())
            .set_name(name.into())
    }
}
