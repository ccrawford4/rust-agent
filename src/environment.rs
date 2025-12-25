use reqwest::Certificate;
use tracing::{debug, info, warn};

pub struct Environment {
    pub openai_api_key: String,
    pub production_mode: bool,
    pub kube_api_server: String, // The URL of the Kubernetes API server
    pub kube_certificate: Option<Certificate>, // The CA certificate for the Kubernetes API server
    pub kube_token: String,      // The Bearer token used to authenticate to the Kubernetes API
    // server
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
            Ok(url) => {
                debug!("KUBE_API_SERVER loaded from environment");
                url
            }
            Err(_) => {
                warn!("KUBE_API_SERVER not found in environment, using default localhost URL");
                "https://localhost:6443".to_string()
            }
        };

        let kube_token = if production_mode {
            debug!("Running in production mode. Getting kube token from mounted token file.");
            match std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token") {
                Ok(token) => {
                    debug!("Kubernetes token loaded from service account");
                    token
                }
                Err(_) => {
                    warn!(
                        "Failed to read Kubernetes token from service account, using empty string"
                    );
                    String::new()
                }
            }
        } else {
            debug!("Not running in production mode, getting temporary token from environment");
            match std::env::var("KUBE_TOKEN") {
                Ok(token) => {
                    debug!("KUBE_TOKEN loaded from environment");
                    token
                }
                Err(_) => {
                    warn!("KUBE_TOKEN not found in environment, using empty string");
                    String::new()
                }
            }
        };

        let kube_certificate = if production_mode {
            debug!("Running in production mode. Getting kube certificate from mounted file.");
            match std::fs::read("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt") {
                Ok(cert_bytes) => match Certificate::from_pem(&cert_bytes) {
                    Ok(cert) => {
                        debug!("Kubernetes CA certificate loaded from service account");
                        Some(cert)
                    }
                    Err(_) => {
                        warn!(
                            "Failed to parse Kubernetes CA certificate from service account, proceeding without certificate"
                        );
                        None
                    }
                },
                Err(_) => {
                    warn!(
                        "Failed to read Kubernetes CA certificate from service account, proceeding without certificate"
                    );
                    None
                }
            }
        } else {
            debug!("Not running in production mode, proceeding without kube certificate");
            None
        };

        Environment {
            openai_api_key,
            production_mode,
            chat_api_key,
            kube_api_server,
            kube_token,
            kube_certificate,
        }
    }
}
