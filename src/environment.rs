use tracing::{debug, info, warn};

pub struct Environment {
    pub openai_api_key: String,
    pub production_mode: bool,
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

        Environment {
            openai_api_key,
            production_mode,
        }
    }
}
