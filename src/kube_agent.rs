use serde::{Deserialize, Serialize};
use tracing::*;

pub struct KubeAgent {
    kube_api_server: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PodMetadata {
    name: String,
    namespace: String,
    uid: String,
    #[serde(rename = "creationTimestamp")]
    creation_timestamp: String,
    labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerSpecPort {
    #[serde(rename = "containerPort")]
    container_port: u16,
    protocol: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerSpec {
    name: String,
    image: String,
    ports: Option<Vec<ContainerSpecPort>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PodCondition {
    #[serde(rename = "type")]
    type_field: String,
    status: String,
    #[serde(rename = "lastProbeTime")]
    last_probe_time: Option<String>,
    #[serde(rename = "lastTransitionTime")]
    last_transition_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PodSpecStatus {
    phase: String,
    conditions: Option<Vec<PodCondition>>,
    #[serde(rename = "startTime")]
    start_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PodSpec {
    containers: Vec<ContainerSpec>,
    #[serde(rename = "nodeName")]
    node_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Pod {
    metadata: PodMetadata,
    spec: Option<PodSpec>,
    status: Option<PodSpecStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PodListResponse {
    items: Vec<Pod>,
}

impl PodListResponse {
    fn as_string(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Found {} pods:\n\n", self.items.len()));

        for (idx, pod) in self.items.iter().enumerate() {
            output.push_str(&format!("Pod {}:\n", idx + 1));
            output.push_str(&format!("  Name: {}\n", pod.metadata.name));
            output.push_str(&format!("  Namespace: {}\n", pod.metadata.namespace));
            output.push_str(&format!("  UID: {}\n", pod.metadata.uid));
            output.push_str(&format!("  Created: {}\n", pod.metadata.creation_timestamp));

            if let Some(labels) = &pod.metadata.labels {
                output.push_str("  Labels:\n");
                for (key, value) in labels {
                    output.push_str(&format!("    {}: {}\n", key, value));
                }
            }

            if let Some(spec) = &pod.spec {
                output.push_str(&format!("  Node: {}\n", spec.node_name.as_deref().unwrap_or("N/A")));
                output.push_str("  Containers:\n");
                for container in &spec.containers {
                    output.push_str(&format!("    - {}\n", container.name));
                    output.push_str(&format!("      Image: {}\n", container.image));
                    if let Some(ports) = &container.ports {
                        output.push_str("      Ports:\n");
                        for port in ports {
                            output.push_str(&format!("        {}:{}\n", port.container_port, port.protocol));
                        }
                    }
                }
            }

            if let Some(status) = &pod.status {
                output.push_str(&format!("  Phase: {}\n", status.phase));
                if let Some(start_time) = &status.start_time {
                    output.push_str(&format!("  Started: {}\n", start_time));
                }
                if let Some(conditions) = &status.conditions {
                    output.push_str("  Conditions:\n");
                    for condition in conditions {
                        output.push_str(&format!("    {}: {}\n", condition.type_field, condition.status));
                    }
                }
            }

            output.push_str("\n");
        }

        output
    }
}

impl KubeAgent {
    pub fn new(kube_api_server: String, token: String) -> Self {
        return KubeAgent {
            kube_api_server,
            token,
        };
    }

    pub async fn get_pods(
        &self,
        namespace: Option<String>,
        limit: Option<u32>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // https://localhost:50220/api/v1/namespaces/default/pods?limit=500
        let mut namespace_path = String::from("default");
        let mut limit_query: u32 = 500;

        // Override defaults if provided
        if let Some(ns) = namespace {
            namespace_path = ns;
        }
        if let Some(lim) = limit {
            limit_query = lim;
        }

        // Construct endpoint
        let endpoint = format!(
            "/api/v1/namespaces/{}/pods?limit={}",
            namespace_path, limit_query
        );

        let response = self.make_request(endpoint).await?;

        // Parse the JSON response
        let pod_list: PodListResponse = serde_json::from_str(&response)?;

        // Return the formatted string
        Ok(pod_list.as_string())
    }

    async fn make_request(&self, endpoint: String) -> Result<String, reqwest::Error> {
        // Connect to the kube api server
        info!(
            "Connecting to Kubernetes API server at {}",
            self.kube_api_server
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // Accept invalid certs for local clusters
            .build()
            .unwrap();

        let request = client
            .get(format!("{}{}", self.kube_api_server, endpoint))
            .send()
            .await;

        match request {
            Ok(resp) => {
                let text = resp.text().await;
                match text {
                    Ok(body) => Ok(body),
                    Err(err) => {
                        error!("Error reading response body: {}", err);
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!("Error sending request to Kubernetes API server: {}", err);
                Err(err)
            }
        }
    }
}

// TODO:
// 1. Connect to minikube and setup a service account and token for the service account to
//    authenticate
//
//    Public Agent:
//
// 2. Update the env + this module to use the token so we can connect to the k8s api server
//    directly (k3s also uses token auth so this method should work for both!)
//
//
//  Some operations we will want (very locked down)
//  - Pods (list the pods and their names)
//  - Cluster info (memory, cpu, etc)
//  - Namespaces (how many, names)
//
//  NO Logs, service accounts, or any other info will be exposed to the agent!
//
//
//  Private agent (long term):
//  - Setup a private agent that can actually modify pods/deployments/etc
//  - Create new deployments
//  - Output the results as yaml/json
//  - Cut a PR to introduce the change!
