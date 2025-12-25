use crate::agent::Agent;
use crate::kube_agent::{KubeAgent, ListPodsTool};
use crate::server::Server;
use dotenv::dotenv;
use environment::Environment;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod agent;
mod environment;
mod kube_agent;
mod server;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize tracing subscriber with environment filter
    // Set RUST_LOG environment variable to control log levels
    // Example: RUST_LOG=debug cargo run
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting SQL Agent application");

    let env = Environment::new();

    let agent = match Agent::new(env.openai_api_key) {
        Ok(agent) => agent,
        Err(e) => {
            error!("Failed to initialize agent: {}", e);
            std::process::exit(1);
        }
    };

    let kube_agent = KubeAgent::new(env.kube_api_server, env.kube_token);
    let list_pods_tool = ListPodsTool::new(kube_agent);
    if let Ok(resp) = list_pods_tool
        .list_pods(Some("default".to_string()), Some(5))
        .await
    {
        info!("Successfully listed pods: {}", resp);
    } else {
        error!("Failed to list pods from Kubernetes API server");
        std::process::exit(1);
    }

    let server = Server::new(agent, "127.0.0.1:8080".to_string(), env.chat_api_key);

    info!("Server initialized, listening on 127.0.0.1:8080");

    if let Err(e) = server.listen().await {
        error!("Failed to start server: {}", e);
        std::process::exit(1);
    }
}
