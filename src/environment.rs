use tracing::{debug, info, warn};

pub struct Environment {
    pub openai_api_key: String,
    pub production_mode: bool,
    pub kube_api_server: String,
    pub kube_token: String,
    pub chat_api_key: String, // The API Key used to enforce limit access to this server to only
                              // authorized users (ie my Next.js portfolio)
}

impl Environment {
    pub fn new() -> Self {
        let openai_api_key = match std::env::var("OPENAI_API_KEY") {
            Ok(key) => {
                debug!("OPENAI_API_KEY loaded from environment");
                key
            }
            Err(_) => {
                warn!("OPENAI_API_KEY not found in environment, using empty string");
                String::new()
            }
        };

        let production_mode = match std::env::var("PRODUCTION_MODE") {
            Ok(val) => {
                let is_production = val.to_lowercase() == "true";
                info!("Production mode: {}", is_production);
                is_production
            }
            Err(_) => {
                debug!("PRODUCTION_MODE not set, defaulting to false");
                false
            }
        };

        let chat_api_key = match std::env::var("CHAT_API_KEY") {
            Ok(key) => {
                debug!("CHAT_API_KEY loaded from environment");
                key
            }
            Err(_) => {
                warn!("CHAT_API_KEY not found in environment, using empty string");
                String::new()
            }
        };

        let kube_api_server = match std::env::var("KUBE_API_SERVER") {
            Ok(server) => {
                debug!("KUBE_API_SERVER loaded from environment");
                server
            }
            Err(_) => {
                warn!("KUBE_API_SERVER not found in environment, using default localhost");
                "https://localhost:6443".to_string()
            }
        };

        let kube_token = match std::env::var("KUBE_TOKEN") {
            Ok(token) => {
                debug!("KUBE_TOKEN loaded from environment");
                token
            }
            Err(_) => {
                warn!("KUBE_TOKEN not found in environment, using empty string");
                String::new()
            }
        };

        Environment {
            openai_api_key,
            production_mode,
            chat_api_key,
            kube_api_server,
            kube_token,
        }
    }
}
