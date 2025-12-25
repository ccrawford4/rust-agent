use reqwest::Error;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::json;
// See pod and container metrics (cpu/memory)
//
// This requires the cluster to enable the metrics-server addon.

use crate::kube::error::KubeAgentError;
use crate::kube::types::{NodeListResponse, NodeMetricsListResponse, NodeMetricsWithUsageResponse};
use crate::kube::KubeAgent;
use tracing::*;

pub struct NodeMetricsTool {
    kube_agent: KubeAgent,
}

impl NodeMetricsTool {
    pub fn new(kube_agent: KubeAgent) -> Self {
        NodeMetricsTool { kube_agent }
    }

    /// Fetch node metrics from the metrics server API
    pub async fn get_node_metrics(&self) -> Result<String, KubeAgentError> {
        let endpoint = String::from("/apis/metrics.k8s.io/v1beta1/nodes");
        let response = self.kube_agent.make_request(endpoint).await?;

        debug!("Kubernetes API response: {}", response);

        Ok(response)
    }

    /// Fetch node information from the core API
    pub async fn get_nodes(&self) -> Result<String, KubeAgentError> {
        let endpoint = String::from("/api/v1/nodes");
        let response = self.kube_agent.make_request(endpoint).await?;

        debug!("Kubernetes API response: {}", response);

        Ok(response)
    }

    /// Fetch both node info and metrics, then combine them to show usage with percentages
    pub async fn get_node_metrics_with_usage(
        &self,
    ) -> Result<NodeMetricsWithUsageResponse, KubeAgentError> {
        // Fetch both APIs in parallel
        let (nodes_response, metrics_response) =
            tokio::join!(self.get_nodes(), self.get_node_metrics());

        let nodes_json = nodes_response?;
        let metrics_json = metrics_response?;

        // Parse the JSON responses
        if let Ok(nodes) = serde_json::from_str(&nodes_json) {
            if let Ok(metrics) = serde_json::from_str::<NodeMetricsListResponse>(&metrics_json) {
                debug!("Parsed Nodes: {:?}", nodes);
                debug!("Parsed Node Metrics: {:?}", metrics);

                // Combine the data
                metrics.combine_with_nodes(&nodes)
            } else {
                Err(KubeAgentError::JsonParseError(Error::new(
                    "Failed to parse node metrics JSON".to_string(),
                )))
            }
        } else {
            Err(KubeAgentError::JsonParseError(
                "Failed to parse nodes or metrics JSON".to_string(),
            ))
        }
    }
}

impl Tool for NodeMetricsTool {
    const NAME: &'static str = "get_node_metrics";
    type Args = ();
    type Output = NodeMetricsWithUsageResponse;
    type Error = KubeAgentError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": Self::NAME,
            "description": "Get node metrics (CPU and memory usage) from the Kubernetes cluster.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .unwrap()
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.get_node_metrics_with_usage().await
    }
}
