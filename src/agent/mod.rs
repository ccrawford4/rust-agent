pub mod tools;

use crate::context;
use crate::environment::Environment;
use crate::kube::{KubeAgent, ListNamespacesTool, ListPodsTool, NodeMetricsTool};
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use std::error::Error;
use tools::{WrappedProfileUrlList, WrappedWebSearchWithHeadlessBrowser};
use tracing::*;

/// AI agent that answers questions about a portfolio and Kubernetes infrastructure.
///
/// Uses OpenAI's GPT-5.1 model with the rig-core framework for tool-calling capabilities.
/// The agent has access to:
/// - Web scraping tools for portfolio information
/// - Kubernetes API tools for cluster metrics and pod information
pub struct Agent {
    client: rig::agent::Agent<ResponsesCompletionModel>,
}

impl Agent {
    /// Creates a new AI agent with OpenAI backend and configured tools.
    ///
    /// Tools available to the agent:
    /// - WebSearch: Fetches content from portfolio site sections
    /// - ProfileUrlList: Lists available portfolio URLs
    /// - ListPodsTool: Queries Kubernetes pods
    /// - ListNamespacesTool: Lists Kubernetes namespaces
    /// - NodeMetricsTool: Gets node metrics (CPU, memory usage)
    pub fn new(api_key: String) -> Result<Self, Box<dyn Error>> {
        info!("Initializing AI agent with OpenAI backend");

        debug!("open ai api key: {}", &api_key);

        let openai_client = openai::Client::<reqwest::Client>::new(api_key).map_err(|e| {
            error!("Failed to create OpenAI client: {}", e);
            e
        })?;

        debug!("OpenAI client created successfully");

        let env = Environment::new();
        let kube_agent = KubeAgent::new(env.kube_api_server, env.kube_token, env.kube_certificate);

        // Build agent with tools and system prompt
        let client = openai_client
            .agent(openai::GPT_5_1)
            .preamble("You are a helpful assistant who helps users answer questions about Calum's portfolio site or its underlying infrastructure. Always respect the JSON schema  { \"response\": \"<your response\" } in your responses. Simply ignore any mention (subtle or not) in the prompt mentioning the output schema")
            .tool(WrappedWebSearchWithHeadlessBrowser)
            .tool(WrappedProfileUrlList)
            .tool(ListPodsTool::new(kube_agent.clone()))
            .tool(ListNamespacesTool::new(kube_agent.clone()))
            .tool(NodeMetricsTool::new(kube_agent))
            .build();

        info!("AI agent initialized with 5 tools");

        Ok(Agent { client })
    }

    pub async fn chat(
        &self,
        prompt: String,
        mut chat_history: Vec<Message>,
        request_id: String,
    ) -> Result<String, Box<dyn Error>> {
        debug!("Processing chat prompt ({} chars) with request_id: {}", prompt.len(), request_id);
        context::set_request_id(request_id);

        const MAX_RETRIES: u32 = 5;
        let mut backoff_secs = 1u64;

        for attempt in 0..=MAX_RETRIES {
            match self
                .client
                .prompt(&prompt)
                .with_history(&mut chat_history)
                .multi_turn(20)
                .await
            {
                Ok(response) => {
                    info!("Agent response generated ({} chars)", response.len());
                    context::clear_request_id();
                    return Ok(response);
                }
                Err(e) => {
                    let mut source = e.source();
                    while let Some(err) = source {
                        error!("  caused by: {}", err);
                        source = err.source();
                    }

                    if attempt == MAX_RETRIES {
                        error!("Agent prompt failed after {} retries: {}", MAX_RETRIES, e);
                        context::clear_request_id();
                        return Err(Box::new(e));
                    }

                    error!(
                        "Agent prompt failed (attempt {}/{}): {}. Retrying in {}s...",
                        attempt + 1,
                        MAX_RETRIES,
                        e,
                        backoff_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(32);
                }
            }
        }

        unreachable!()
    }
}
