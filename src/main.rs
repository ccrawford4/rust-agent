use crate::agent::WebSearch;
use environment::Environment;
use rig::{client::CompletionClient, completion::Prompt, providers::openai};

mod agent;
mod environment;

#[tokio::main]
async fn main() {
    let env: Environment = Environment::new();
    let openai_client = openai::Client::<reqwest::Client>::new(env.openai_api_key)
        .expect("Error! Could not initialize OpenAI Client");

    let gpt4 = openai_client
        .agent(openai::GPT_5_1)
        .preamble("You are a helpful assistant.")
        .tool(WebSearch)
        .build();

    let response = gpt4
        .prompt(
            "Find information about calum and report it to me. (Hint: it may be on a test server)",
        )
        .await
        .expect("Failed to prompt GPT-4");

    println!("GPT-4: {response}")
}
