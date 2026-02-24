// Copyright 2026 Google LLC
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

use google_cloud_api::model::MonitoredResource;
use opentelemetry::{Key, Value};
use opentelemetry_sdk::Resource;
use std::collections::HashMap;

fn get_attr(resource: &Resource, key: &'static str) -> Option<String> {
    match resource.get(&Key::from_static_str(key))? {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    }
}

fn first_attr(resource: &Resource, keys: &[&'static str]) -> Option<String> {
    for k in keys {
        if let Some(value) = resource.get(&Key::from_static_str(k)) {
            match value {
                Value::String(s) if !s.as_str().is_empty() => return Some(s.to_string()),
                _ => continue,
            }
        }
    }
    None
}

/// Maps an OpenTelemetry `Resource` to a Google Cloud `MonitoredResource`.
pub fn map_resource(resource: &Resource) -> MonitoredResource {
    let mut labels = HashMap::new();

    let cloud_platform = get_attr(resource, "cloud.platform");
    let has_k8s_cluster = get_attr(resource, "k8s.cluster.name").is_some();

    let r_type = if cloud_platform.as_deref() == Some("gcp_app_engine") {
        "gae_instance"
    } else if cloud_platform.as_deref() == Some("gcp_compute_engine") {
        "gce_instance"
    } else if cloud_platform.as_deref() == Some("aws_ec2") {
        "aws_ec2_instance"
    } else if cloud_platform.as_deref() == Some("gcp_bare_metal_solution") {
        "baremetalsolution.googleapis.com/Instance"
    } else if cloud_platform.as_deref() == Some("gcp_kubernetes_engine") || has_k8s_cluster {
        if get_attr(resource, "k8s.container.name").is_some() {
            "k8s_container"
        } else if get_attr(resource, "k8s.pod.name").is_some() {
            "k8s_pod"
        } else if get_attr(resource, "k8s.node.name").is_some() {
            "k8s_node"
        } else {
            "k8s_cluster"
        }
    } else if (get_attr(resource, "service.name").is_some()
        || get_attr(resource, "faas.name").is_some())
        && (get_attr(resource, "service.instance.id").is_some()
            || get_attr(resource, "faas.instance").is_some())
    {
        "generic_task"
    } else {
        "generic_node"
    };

    match r_type {
        "gce_instance" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone"]) {
                labels.insert("zone".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["host.id"]) {
                labels.insert("instance_id".to_string(), v);
            }
        }
        "k8s_container" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.cluster.name"]) {
                labels.insert("cluster_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.namespace.name"]) {
                labels.insert("namespace_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.pod.name"]) {
                labels.insert("pod_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.container.name"]) {
                labels.insert("container_name".to_string(), v);
            }
        }
        "k8s_pod" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.cluster.name"]) {
                labels.insert("cluster_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.namespace.name"]) {
                labels.insert("namespace_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.pod.name"]) {
                labels.insert("pod_name".to_string(), v);
            }
        }
        "k8s_node" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.cluster.name"]) {
                labels.insert("cluster_name".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.node.name"]) {
                labels.insert("node_name".to_string(), v);
            }
        }
        "k8s_cluster" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["k8s.cluster.name"]) {
                labels.insert("cluster_name".to_string(), v);
            }
        }
        "gae_instance" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["faas.name"]) {
                labels.insert("module_id".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["faas.version"]) {
                labels.insert("version_id".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["faas.instance"]) {
                labels.insert("instance_id".to_string(), v);
            }
        }
        "aws_ec2_instance" => {
            if let Some(v) = first_attr(resource, &["host.id"]) {
                labels.insert("instance_id".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("region".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["cloud.account.id"]) {
                labels.insert("aws_account".to_string(), v);
            }
        }
        "baremetalsolution.googleapis.com/Instance" => {
            if let Some(v) = first_attr(resource, &["cloud.availability_zone", "cloud.region"]) {
                labels.insert("location".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["cloud.account.id"]) {
                labels.insert("project_id".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["host.id"]) {
                labels.insert("instance_id".to_string(), v);
            }
        }
        "generic_task" => {
            let loc = first_attr(resource, &["cloud.availability_zone", "cloud.region"])
                .unwrap_or_else(|| "global".to_string());
            labels.insert("location".to_string(), loc);

            if let Some(v) = first_attr(resource, &["service.namespace"]) {
                labels.insert("namespace".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["service.name", "faas.name"]) {
                labels.insert("job".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["service.instance.id", "faas.instance"]) {
                labels.insert("task_id".to_string(), v);
            }
        }
        _ => {
            let loc = first_attr(resource, &["cloud.availability_zone", "cloud.region"])
                .unwrap_or_else(|| "global".to_string());
            labels.insert("location".to_string(), loc);

            if let Some(v) = first_attr(resource, &["service.namespace"]) {
                labels.insert("namespace".to_string(), v);
            }
            if let Some(v) = first_attr(resource, &["host.id", "host.name"]) {
                labels.insert("node_id".to_string(), v);
            }
        }
    }

    MonitoredResource::new().set_type(r_type).set_labels(labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{KeyValue, StringValue, Value};
    use opentelemetry_sdk::Resource;
    use test_case::test_case;

    fn make_resource(attrs: &[(&'static str, &'static str)]) -> Resource {
        let kvs = attrs
            .iter()
            .map(|(k, v)| KeyValue::new(*k, Value::String(StringValue::from(*v))))
            .collect::<Vec<_>>();
        Resource::builder_empty().with_attributes(kvs).build()
    }

    #[track_caller]
    fn assert_resource_mapping(
        expected_type: &str,
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        let resource = make_resource(attrs);
        let mr = map_resource(&resource);
        assert_eq!(mr.r#type, expected_type);
        for (k, v) in expected_labels {
            assert_eq!(mr.labels.get(*k).map(|s| s.as_str()), Some(*v));
        }
        assert_eq!(mr.labels.len(), expected_labels.len());
    }

    #[test_case(&[
        ("cloud.platform", "gcp_compute_engine"),
        ("cloud.availability_zone", "us-central1-a"),
        ("host.id", "12345")
    ], &[
        ("zone", "us-central1-a"),
        ("instance_id", "12345")
    ] ; "gce_instance basic")]
    fn test_gce_instance(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("gce_instance", attrs, expected_labels);
    }

    #[test_case(&[
        ("cloud.platform", "gcp_kubernetes_engine"),
        ("cloud.availability_zone", "z1"),
        ("k8s.cluster.name", "c1"),
        ("k8s.namespace.name", "ns1"),
        ("k8s.pod.name", "p1"),
        ("k8s.container.name", "cont1")
    ], &[
        ("location", "z1"),
        ("cluster_name", "c1"),
        ("namespace_name", "ns1"),
        ("pod_name", "p1"),
        ("container_name", "cont1")
    ] ; "k8s_container zone")]
    #[test_case(&[
        ("cloud.platform", "gcp_kubernetes_engine"),
        ("cloud.region", "r1"),
        ("k8s.cluster.name", "c1"),
        ("k8s.namespace.name", "ns1"),
        ("k8s.pod.name", "p1"),
        ("k8s.container.name", "cont1")
    ], &[
        ("location", "r1"),
        ("cluster_name", "c1"),
        ("namespace_name", "ns1"),
        ("pod_name", "p1"),
        ("container_name", "cont1")
    ] ; "k8s_container region fallback")]
    fn test_k8s_container(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("k8s_container", attrs, expected_labels);
    }

    #[test_case(&[
        ("k8s.cluster.name", "c1"),
        ("k8s.pod.name", "p1"),
        ("cloud.availability_zone", "z1")
    ], &[
        ("location", "z1"),
        ("cluster_name", "c1"),
        ("pod_name", "p1")
    ] ; "k8s_pod")]
    fn test_k8s_pod(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("k8s_pod", attrs, expected_labels);
    }

    #[test_case(&[
        ("k8s.cluster.name", "c1"),
        ("k8s.node.name", "n1"),
        ("cloud.availability_zone", "z1")
    ], &[
        ("location", "z1"),
        ("cluster_name", "c1"),
        ("node_name", "n1")
    ] ; "k8s_node")]
    fn test_k8s_node(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("k8s_node", attrs, expected_labels);
    }

    #[test_case(&[
        ("k8s.cluster.name", "c1"),
        ("cloud.availability_zone", "z1")
    ], &[
        ("location", "z1"),
        ("cluster_name", "c1")
    ] ; "k8s_cluster")]
    fn test_k8s_cluster(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("k8s_cluster", attrs, expected_labels);
    }

    #[test_case(&[
        ("cloud.platform", "gcp_app_engine"),
        ("cloud.availability_zone", "z1"),
        ("faas.name", "f1"),
        ("faas.version", "v1"),
        ("faas.instance", "i1")
    ], &[
        ("location", "z1"),
        ("module_id", "f1"),
        ("version_id", "v1"),
        ("instance_id", "i1")
    ] ; "gae_instance zone")]
    #[test_case(&[
        ("cloud.platform", "gcp_app_engine"),
        ("cloud.region", "r1"),
        ("faas.name", "f1"),
        ("faas.version", "v1"),
        ("faas.instance", "i1")
    ], &[
        ("location", "r1"),
        ("module_id", "f1"),
        ("version_id", "v1"),
        ("instance_id", "i1")
    ] ; "gae_instance region fallback")]
    fn test_gae_instance(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("gae_instance", attrs, expected_labels);
    }

    #[test_case(&[
        ("cloud.platform", "aws_ec2"),
        ("host.id", "h1"),
        ("cloud.availability_zone", "z1"),
        ("cloud.account.id", "a1")
    ], &[
        ("instance_id", "h1"),
        ("region", "z1"),
        ("aws_account", "a1")
    ] ; "aws_ec2_instance zone")]
    #[test_case(&[
        ("cloud.platform", "aws_ec2"),
        ("host.id", "h1"),
        ("cloud.region", "r1"),
        ("cloud.account.id", "a1")
    ], &[
        ("instance_id", "h1"),
        ("region", "r1"),
        ("aws_account", "a1")
    ] ; "aws_ec2_instance region fallback")]
    fn test_aws_ec2_instance(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("aws_ec2_instance", attrs, expected_labels);
    }

    #[test_case(&[
        ("cloud.platform", "gcp_bare_metal_solution"),
        ("cloud.availability_zone", "z1"),
        ("cloud.account.id", "a1"),
        ("host.id", "h1")
    ], &[
        ("location", "z1"),
        ("project_id", "a1"),
        ("instance_id", "h1")
    ] ; "baremetalsolution zone")]
    #[test_case(&[
        ("cloud.platform", "gcp_bare_metal_solution"),
        ("cloud.region", "r1"),
        ("cloud.account.id", "a1"),
        ("host.id", "h1")
    ], &[
        ("location", "r1"),
        ("project_id", "a1"),
        ("instance_id", "h1")
    ] ; "baremetalsolution region fallback")]
    fn test_baremetalsolution_instance(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping(
            "baremetalsolution.googleapis.com/Instance",
            attrs,
            expected_labels,
        );
    }

    #[test_case(&[
        ("service.name", "s1"),
        ("service.instance.id", "i1"),
        ("service.namespace", "ns1"),
        ("cloud.availability_zone", "z1")
    ], &[
        ("location", "z1"),
        ("namespace", "ns1"),
        ("job", "s1"),
        ("task_id", "i1")
    ] ; "generic_task service zone")]
    #[test_case(&[
        ("faas.name", "f1"),
        ("faas.instance", "i1"),
        ("cloud.region", "r1")
    ], &[
        ("location", "r1"),
        ("job", "f1"),
        ("task_id", "i1")
    ] ; "generic_task faas region fallback")]
    #[test_case(&[
        ("service.name", "s1"),
        ("faas.instance", "i1")
    ], &[
        ("location", "global"),
        ("job", "s1"),
        ("task_id", "i1")
    ] ; "generic_task global fallback mixed job instance")]
    fn test_generic_task(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("generic_task", attrs, expected_labels);
    }

    #[test_case(&[
        ("host.id", "h1"),
        ("service.namespace", "ns1"),
        ("cloud.availability_zone", "z1")
    ], &[
        ("location", "z1"),
        ("namespace", "ns1"),
        ("node_id", "h1")
    ] ; "generic_node host.id zone")]
    #[test_case(&[
        ("host.name", "h2"),
        ("cloud.region", "r1")
    ], &[
        ("location", "r1"),
        ("node_id", "h2")
    ] ; "generic_node host.name region fallback")]
    #[test_case(&[], &[
        ("location", "global")
    ] ; "generic_node global fallback empty")]
    fn test_generic_node(
        attrs: &[(&'static str, &'static str)],
        expected_labels: &[(&'static str, &'static str)],
    ) {
        assert_resource_mapping("generic_node", attrs, expected_labels);
    }
}
