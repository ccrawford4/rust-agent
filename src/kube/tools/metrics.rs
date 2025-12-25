use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
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
    pub async fn get_node_metrics(&self) -> Result<NodeMetricsListResponse, KubeAgentError> {
        let endpoint = String::from("/apis/metrics.k8s.io/v1beta1/nodes");
        let response = self.kube_agent.make_request(endpoint).await?;

        let metrics: NodeMetricsListResponse = serde_json::from_str(&response).map_err(|e| {
            error!("Error parsing node metrics JSON response: {}", e);
            KubeAgentError::from(e)
        })?;

        Ok(metrics)
    }

    /// Fetch node information from the core API
    pub async fn get_nodes(&self) -> Result<NodeListResponse, KubeAgentError> {
        let endpoint = String::from("/api/v1/nodes");
        let response = self.kube_agent.make_request(endpoint).await?;

        debug!("Kubernetes API response: {}", response);

        let nodes: NodeListResponse = serde_json::from_str(&response).map_err(|e| {
            error!("Error parsing nodes JSON response: {}", e);
            KubeAgentError::from(e)
        })?;

        Ok(nodes)
    }

    /// Fetch both node info and metrics, then combine them to show usage with percentages
    pub async fn get_node_metrics_with_usage(
        &self,
    ) -> Result<NodeMetricsWithUsageResponse, KubeAgentError> {
        // Fetch both APIs in parallel
        let (nodes_result, metrics_result) =
            tokio::join!(self.get_nodes(), self.get_node_metrics());

        let nodes = nodes_result?;
        let metrics = metrics_result?;

        debug!("Parsed Nodes: {:?}", nodes);
        debug!("Parsed Node Metrics: {:?}", metrics);

        // Combine the data
        metrics.combine_with_nodes(&nodes)
    }
}

#[derive(Serialize, Deserialize)]
pub struct NodeMetricsToolArgs {}

impl Tool for NodeMetricsTool {
    const NAME: &'static str = "get_node_metrics";
    type Args = NodeMetricsToolArgs;
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
