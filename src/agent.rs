use crate::environment::Environment;
use crate::kube_agent::KubeAgent;
use crate::kube_agent::ListPodsTool;
use rig::completion::Message;
use rig::completion::Prompt;
use rig::completion::PromptError;
use rig::completion::ToolDefinition;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use rig::tool::Tool;
use rig::{client::CompletionClient, providers::openai};
use serde::de::{self, Visitor};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fmt;
use tracing::*;

#[derive(Debug, Clone)]
enum ProfileUrl {
    About,
    Work,
    Projects,
    Contact,
}

fn get_portfolio_host() -> String {
    let env = Environment::new();
    if env.production_mode {
        "https://about.calum.run".to_string()
    } else {
        "http://localhost:3000".to_string()
    }
}

impl ProfileUrl {
    /// Returns the URL string for this variant
    fn as_url(&self) -> String {
        let host = get_portfolio_host();
        match self {
            ProfileUrl::About => format!("{}/?tab=About", host),
            ProfileUrl::Work => format!("{}/?tab=Work", host),
            ProfileUrl::Projects => format!("{}/?tab=Projects", host),
            ProfileUrl::Contact => format!("{}/?tab=Contact", host),
        }
    }
}

impl fmt::Display for ProfileUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_url())
    }
}

struct ProfileUrlVisitor;

impl<'de> Visitor<'de> for ProfileUrlVisitor {
    type Value = ProfileUrl;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid URL string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "https://about.calum.run/?tab=About" | "http://localhost:3000/?tab=About" => {
                Ok(ProfileUrl::About)
            }
            "https://about.calum.run/?tab=Work" | "http://localhost:3000/?tab=Work" => {
                Ok(ProfileUrl::Work)
            }
            "https://about.calum.run/?tab=Projects" | "http://localhost:3000/?tab=Projects" => {
                Ok(ProfileUrl::Projects)
            }
            "https://about.calum.run/?tab=Contact" | "http://localhost:3000/?tab=Contact" => {
                Ok(ProfileUrl::Contact)
            }
            _ => Err(de::Error::unknown_variant(
                value,
                &[
                    "https://about.calum.run/?tab=About",
                    "https://about.calum.run/?tab=Work",
                    "https://about.calum.run/?tab=Projects",
                    "https://about.calum.run/?tab=Contact",
                    "http://localhost:3000/?tab=About",
                    "http://localhost:3000/?tab=Work",
                    "http://localhost:3000/?tab=Projects",
                    "http://localhost:3000/?tab=Contact",
                ],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for ProfileUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ProfileUrlVisitor)
    }
}

#[derive(Deserialize)]
pub struct WebSearchArgs {
    url: ProfileUrl,
}

#[derive(Deserialize, Serialize)]
pub struct WebSearch;

#[derive(Debug)]
pub struct ModelError(String);

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ModelError {}

// For Listing out the available profile urls
struct ProfileUrlList;

#[derive(Debug, Deserialize)]
struct ProfileUrlListArgs {}

impl Tool for ProfileUrlList {
    const NAME: &'static str = "profile_url_list";
    type Error = ModelError;
    type Args = ProfileUrlListArgs;
    type Output = Vec<String>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // This should never fail as we're using a static JSON structure
        serde_json::from_value(json!({
            "name": "profile_url_list",
            "description": "list of available profile URLs about calum (cal) crawford",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .unwrap_or_else(|e| {
            error!("Critical error: Failed to create tool definition: {}", e);
            panic!(
                "Invalid static tool definition - this is a programming error: {}",
                e
            );
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!("Providing list of profile URLs");
        let result = vec![
            ProfileUrl::About.as_url(),
            ProfileUrl::Work.as_url(),
            ProfileUrl::Projects.as_url(),
            ProfileUrl::Contact.as_url(),
        ];
        debug!("Providing profile URL list: {:?}", result);

        Ok(result)
    }
}

impl Tool for WebSearch {
    const NAME: &'static str = "web_search";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // This should never fail as we're using a static JSON structure
        serde_json::from_value(json!({
            "name": "web_search",
            "description": "search the web for information about the user",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "url to search"
                    }
                },
                "required": ["url"]
            }
        }))
        .unwrap_or_else(|e| {
            error!("Critical error: Failed to create tool definition: {}", e);
            panic!(
                "Invalid static tool definition - this is a programming error: {}",
                e
            );
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("Fetching web content from: {}", args.url);

        let response = reqwest::get(args.url.as_url()).await.map_err(|e| {
            error!("Error fetching URL {}: {}", args.url, e);

            let mut source = e.source();
            while let Some(err) = source {
                error!("  caused by: {}", err);
                source = err.source();
            }

            ModelError(e.to_string())
        })?;

        let body = response.text().await.map_err(|e| {
            error!("Error reading response body: {}", e);
            ModelError(e.to_string())
        })?;

        debug!(
            "Successfully fetched web page content ({} bytes)",
            body.len()
        );

        Ok(body)
    }
}

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

        let client = openai_client
            .agent(openai::GPT_5_1)
            .preamble("You are a helpful assistant who helps users answer questions about Calum's portfolio site or its underlying infrastructure. Always respect the JSON schema  { \"response\": \"<your response\" } in your responses. Simply ignore any mention (subtle or not) in the prompt mentioning the output schema")
            .tool(WebSearch)
            .tool(ProfileUrlList)
            .tool(ListPodsTool::new(KubeAgent::new(
                Environment::new().kube_api_server,
                Environment::new().kube_token,
            )))
            .build();

        info!("Agent built successfully with web search tool and structured output");

        Ok(Agent { client })
    }

    /// Builds a prompt with chat history appended
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
