use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use tracing::*;

#[derive(Debug)]
pub enum KubeAgentError {
    HttpError(reqwest::Error),
    JsonParseError(serde_json::Error),
    ApiError(String),
}

impl fmt::Display for KubeAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KubeAgentError::HttpError(err) => write!(f, "HTTP request error: {}", err),
            KubeAgentError::JsonParseError(err) => write!(f, "JSON parsing error: {}", err),
            KubeAgentError::ApiError(msg) => write!(f, "Kubernetes API error: {}", msg),
        }
    }
}

impl std::error::Error for KubeAgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KubeAgentError::HttpError(err) => Some(err),
            KubeAgentError::JsonParseError(err) => Some(err),
            KubeAgentError::ApiError(_) => None,
        }
    }
}

impl From<reqwest::Error> for KubeAgentError {
    fn from(err: reqwest::Error) -> Self {
        KubeAgentError::HttpError(err)
    }
}

impl From<serde_json::Error> for KubeAgentError {
    fn from(err: serde_json::Error) -> Self {
        KubeAgentError::JsonParseError(err)
    }
}

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
                output.push_str(&format!(
                    "  Node: {}\n",
                    spec.node_name.as_deref().unwrap_or("N/A")
                ));
                output.push_str("  Containers:\n");
                for container in &spec.containers {
                    output.push_str(&format!("    - {}\n", container.name));
                    output.push_str(&format!("      Image: {}\n", container.image));
                    if let Some(ports) = &container.ports {
                        output.push_str("      Ports:\n");
                        for port in ports {
                            output.push_str(&format!(
                                "        {}:{}\n",
                                port.container_port, port.protocol
                            ));
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
                        output.push_str(&format!(
                            "    {}: {}\n",
                            condition.type_field, condition.status
                        ));
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

    async fn make_request(&self, endpoint: String) -> Result<String, KubeAgentError> {
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
                        Err(KubeAgentError::from(err))
                    }
                }
            }
            Err(err) => {
                error!("Error sending request to Kubernetes API server: {}", err);
                Err(KubeAgentError::from(err))
            }
        }
    }
}

pub struct ListPodsTool {
    kube_agent: KubeAgent,
}

impl ListPodsTool {
    pub fn new(kube_agent: KubeAgent) -> Self {
        ListPodsTool { kube_agent }
    }

    pub async fn list_pods(
        &self,
        namespace: Option<String>,
        limit: Option<u32>,
    ) -> Result<String, KubeAgentError> {
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

        let response = self.kube_agent.make_request(endpoint).await?;

        debug!("Kubernetes API response: {}", response);

        // Parse the JSON response
        let pod_list: PodListResponse = serde_json::from_str(&response).map_err(|e| {
            error!("Error parsing JSON response: {}", e);
            KubeAgentError::from(e)
        })?;

        // Return the formatted string
        Ok(pod_list.as_string())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListPodsToolArgs {
    pub namespace: Option<String>,
    pub limit: Option<u32>,
}

impl Tool for ListPodsTool {
    const NAME: &'static str = "list_pods";
    type Args = ListPodsToolArgs;
    type Output = String;
    type Error = KubeAgentError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "list_pods",
            "description": "List pods in a Kubernetes cluster namespace",
            "parameters": {
                "type": "object",
                "properties": {
                    "namespace": {
                        "type": "string",
                        "description": "The namespace to list pods from (default is 'default')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of pods to return (default is 500)"
                    }
                },
                "required": []
            }
        }))
        .unwrap()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.list_pods(args.namespace, args.limit).await
    }
}
