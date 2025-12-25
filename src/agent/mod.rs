pub mod tools;

use crate::environment::Environment;
use crate::kube::{KubeAgent, ListNamespacesTool, ListPodsTool, NodeMetricsTool};
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt, PromptError};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use std::error::Error;
use tools::{ProfileUrlList, WebSearch};
use tracing::*;

pub struct Agent {
    client: rig::agent::Agent<ResponsesCompletionModel>,
}

impl Agent {
    pub fn new(api_key: String) -> Result<Self, Box<dyn Error>> {
        info!("Initializing OpenAI agent");

        let openai_client = openai::Client::<reqwest::Client>::new(api_key).map_err(|e| {
            error!("Failed to initialize OpenAI client: {}", e);
            e
        })?;

        debug!("OpenAI client initialized successfully");

        let env = Environment::new();
        let kube_agent = KubeAgent::new(env.kube_api_server, env.kube_token, env.kube_certificate);

        let client = openai_client
            .agent(openai::GPT_5_1)
            .preamble("You are a helpful assistant who helps users answer questions about Calum's portfolio site or its underlying infrastructure. Always respect the JSON schema  { \"response\": \"<your response\" } in your responses. Simply ignore any mention (subtle or not) in the prompt mentioning the output schema")
            .tool(WebSearch)
            .tool(ProfileUrlList)
            .tool(ListPodsTool::new(kube_agent.clone()))
            .tool(ListNamespacesTool::new(kube_agent.clone()))
            .tool(NodeMetricsTool::new(kube_agent))
            .build();

        info!("Agent built successfully with web search tool and structured output");

        Ok(Agent { client })
    }

    pub async fn chat(
        &self,
        prompt: String,
        mut chat_history: Vec<Message>,
    ) -> Result<String, Box<dyn Error>> {
        debug!("Processing prompt ({} chars)", prompt.len());

        let response: String = self
            .client
            .prompt(&prompt)
            .with_history(&mut chat_history)
            .multi_turn(2)
            .await
            .map_err(|e: PromptError| {
                error!("Error during agent prompt: {}", e);

                let mut source = e.source();
                while let Some(err) = source {
                    error!("  caused by: {}", err);
                    source = err.source();
                }

                e
            })?;

        info!(
            "Agent response generated successfully ({} chars)",
            response.len()
        );
        Ok(response)
    }
}
