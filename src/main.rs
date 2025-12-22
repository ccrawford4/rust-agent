use crate::agent::Agent;
use crate::server::Server;
use environment::Environment;
mod agent;
mod environment;
mod server;

#[tokio::main]
async fn main() {
    let env = Environment::new();

    let agent = Agent::new(env.openai_api_key);

    let server = Server::new(agent, "127.0.0.1:8080".to_string());
    server.listen().await.expect("Failed to start server");
}
