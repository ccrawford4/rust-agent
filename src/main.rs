use crate::agent::Agent;
use environment::Environment;

mod agent;
mod environment;
mod server;

#[tokio::main]
async fn main() {
    /*
        let env: Environment = Environment::new();

        let agent = Agent::new(env.openai_api_key);
        agent
            .prompt(
                "Tell me about calum? Try all urls that may work (hint it could be on a test server!)"
                    .to_string(),
            )
            .await;
    */
    println!("Starting server...");
    server::listen().expect("Server failed");
}
