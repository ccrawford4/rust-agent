use rig::completion::ToolDefinition;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use rig::tool::Tool;
use rig::{client::CompletionClient, completion::Prompt, providers::openai};
use serde::de::{self, Visitor};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
enum Url {
    Home,
    About,
    Work,
    Test,
}

impl Url {
    /// Returns the URL string for this variant
    fn as_str(&self) -> &'static str {
        match self {
            Url::Home => "https://home.calum.run",
            Url::About => "https://about.calum.run",
            Url::Work => "https://work.calum.run",
            Url::Test => "http://localhost:3000",
        }
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

struct UrlVisitor;

impl<'de> Visitor<'de> for UrlVisitor {
    type Value = Url;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid URL string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "https://home.calum.run" => Ok(Url::Home),
            "https://about.calum.run" => Ok(Url::About),
            "https://work.calum.run" => Ok(Url::Work),
            "http://localhost:3000" => Ok(Url::Test),
            _ => Err(de::Error::unknown_variant(
                value,
                &[
                    "https://home.calum.run",
                    "https://about.calum.run",
                    "https://work.calum.run",
                    "http://localhost:3000",
                ],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(UrlVisitor)
    }
}

#[derive(Deserialize)]
pub struct WebSearchArgs {
    url: Url,
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

/*
 * Continue with this example: https://docs.rig.rs/docs/concepts/tools
 *
 */

impl Tool for WebSearch {
    const NAME: &'static str = "web_search";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "web_search",
            "description": "search the web for information about the user",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "url to search - one of: http://localhost:3000, https://home.calum.run, https://about.calum.run, https://work.calum.run"
                    }
                },
                "required": ["url"]
            }
        }))
        .expect("Failed to create tool definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let response = reqwest::get(args.url.as_str()).await.map_err(|e| {
            println!("Error fetching URL: {}", e);

            let mut source = e.source();
            while let Some(err) = source {
                println!("  caused by: {}", err);
                source = err.source();
            }

            ModelError(e.to_string())
        })?;
        let body = response
            .text()
            .await
            .map_err(|e| ModelError(e.to_string()))?;

        println!("Web page content:\n{}", body);

        Ok(body)
    }
}

pub struct Agent {
    client: rig::agent::Agent<ResponsesCompletionModel>,
}

impl Agent {
    pub fn new(api_key: String) -> Self {
        let openai_client = openai::Client::<reqwest::Client>::new(api_key)
            .expect("Error! Could not initialize OpenAI Client");

        let client = openai_client
            .agent(openai::GPT_5_1)
            .preamble("You are a helpful assistant.")
            .tool(WebSearch)
            .build();

        return Agent { client };
    }

    pub async fn prompt(&self, prompt: String) {
        // Use the agent to process the prompt
        let response = self
            .client
            .prompt(prompt)
            .await
            .expect("Failed to prompt GPT-4");
        println!("Agent response: {}", response);
    }
}
