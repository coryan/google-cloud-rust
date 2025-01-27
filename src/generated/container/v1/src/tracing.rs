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

/// Implements a [ClusterManager](crate::traits::) decorator for logging and tracing.
#[derive(Clone, Debug)]
pub struct ClusterManager<T>
where
    T: crate::traits::ClusterManager + std::fmt::Debug + Send + Sync,
{
    inner: T,
}

impl<T> ClusterManager<T>
where
    T: crate::traits::ClusterManager + std::fmt::Debug + Send + Sync,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> crate::traits::ClusterManager for ClusterManager<T>
where
    T: crate::traits::ClusterManager + std::fmt::Debug + Send + Sync,
{
    #[tracing::instrument(ret)]
    async fn list_clusters(
        &self,
        req: crate::model::ListClustersRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::ListClustersResponse> {
        self.inner.list_clusters(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn get_cluster(
        &self,
        req: crate::model::GetClusterRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Cluster> {
        self.inner.get_cluster(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn create_cluster(
        &self,
        req: crate::model::CreateClusterRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.create_cluster(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn update_cluster(
        &self,
        req: crate::model::UpdateClusterRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.update_cluster(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn update_node_pool(
        &self,
        req: crate::model::UpdateNodePoolRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.update_node_pool(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_node_pool_autoscaling(
        &self,
        req: crate::model::SetNodePoolAutoscalingRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_node_pool_autoscaling(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_logging_service(
        &self,
        req: crate::model::SetLoggingServiceRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_logging_service(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_monitoring_service(
        &self,
        req: crate::model::SetMonitoringServiceRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_monitoring_service(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_addons_config(
        &self,
        req: crate::model::SetAddonsConfigRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_addons_config(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_locations(
        &self,
        req: crate::model::SetLocationsRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_locations(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn update_master(
        &self,
        req: crate::model::UpdateMasterRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.update_master(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_master_auth(
        &self,
        req: crate::model::SetMasterAuthRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_master_auth(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn delete_cluster(
        &self,
        req: crate::model::DeleteClusterRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.delete_cluster(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn list_operations(
        &self,
        req: crate::model::ListOperationsRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::ListOperationsResponse> {
        self.inner.list_operations(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn get_operation(
        &self,
        req: crate::model::GetOperationRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.get_operation(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn cancel_operation(
        &self,
        req: crate::model::CancelOperationRequest,
        options: gax::options::RequestOptions,
    ) -> Result<wkt::Empty> {
        self.inner.cancel_operation(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn get_server_config(
        &self,
        req: crate::model::GetServerConfigRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::ServerConfig> {
        self.inner.get_server_config(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn get_json_web_keys(
        &self,
        req: crate::model::GetJSONWebKeysRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::GetJSONWebKeysResponse> {
        self.inner.get_json_web_keys(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn list_node_pools(
        &self,
        req: crate::model::ListNodePoolsRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::ListNodePoolsResponse> {
        self.inner.list_node_pools(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn get_node_pool(
        &self,
        req: crate::model::GetNodePoolRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::NodePool> {
        self.inner.get_node_pool(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn create_node_pool(
        &self,
        req: crate::model::CreateNodePoolRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.create_node_pool(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn delete_node_pool(
        &self,
        req: crate::model::DeleteNodePoolRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.delete_node_pool(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn complete_node_pool_upgrade(
        &self,
        req: crate::model::CompleteNodePoolUpgradeRequest,
        options: gax::options::RequestOptions,
    ) -> Result<wkt::Empty> {
        self.inner.complete_node_pool_upgrade(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn rollback_node_pool_upgrade(
        &self,
        req: crate::model::RollbackNodePoolUpgradeRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.rollback_node_pool_upgrade(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_node_pool_management(
        &self,
        req: crate::model::SetNodePoolManagementRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_node_pool_management(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_labels(
        &self,
        req: crate::model::SetLabelsRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_labels(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_legacy_abac(
        &self,
        req: crate::model::SetLegacyAbacRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_legacy_abac(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn start_ip_rotation(
        &self,
        req: crate::model::StartIPRotationRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.start_ip_rotation(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn complete_ip_rotation(
        &self,
        req: crate::model::CompleteIPRotationRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.complete_ip_rotation(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_node_pool_size(
        &self,
        req: crate::model::SetNodePoolSizeRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_node_pool_size(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_network_policy(
        &self,
        req: crate::model::SetNetworkPolicyRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_network_policy(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn set_maintenance_policy(
        &self,
        req: crate::model::SetMaintenancePolicyRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::Operation> {
        self.inner.set_maintenance_policy(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn list_usable_subnetworks(
        &self,
        req: crate::model::ListUsableSubnetworksRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::ListUsableSubnetworksResponse> {
        self.inner.list_usable_subnetworks(req, options).await
    }

    #[tracing::instrument(ret)]
    async fn check_autopilot_compatibility(
        &self,
        req: crate::model::CheckAutopilotCompatibilityRequest,
        options: gax::options::RequestOptions,
    ) -> Result<crate::model::CheckAutopilotCompatibilityResponse> {
        self.inner.check_autopilot_compatibility(req, options).await
    }
}