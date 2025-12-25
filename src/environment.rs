use reqwest::Certificate;
use tracing::{debug, info, warn};

/// Application configuration loaded from environment variables.
///
/// Handles different configuration sources based on deployment mode:
/// - Local development: loads from .env file and environment variables
/// - Production (Kubernetes): loads from mounted secrets and service account tokens
pub struct Environment {
    /// OpenAI API key for AI agent functionality
    pub openai_api_key: String,

    /// Whether the application is running in production mode (affects token/cert loading)
    pub production_mode: bool,

    /// Kubernetes API server URL
    pub kube_api_server: String,

    /// CA certificate for secure Kubernetes API communication (production only)
    pub kube_certificate: Option<Certificate>,

    /// Bearer token for Kubernetes API authentication
    pub kube_token: String,

    /// API key for authenticating requests to this server
    pub chat_api_key: String,
}

impl Environment {
    /// Creates a new Environment by loading configuration from environment variables.
    ///
    /// In production mode (PRODUCTION_MODE=true):
    /// - Loads Kubernetes credentials from mounted service account files
    /// - Uses CA certificates for secure cluster communication
    ///
    /// In development mode:
    /// - Loads all credentials from environment variables
    /// - Accepts self-signed certificates for local clusters
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
            debug!("Production mode: loading Kubernetes token from mounted service account");
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
            debug!("Development mode: loading Kubernetes token from KUBE_TOKEN environment variable");
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
            debug!("Production mode: loading Kubernetes CA certificate from mounted service account");
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
            debug!("Development mode: skipping CA certificate (will accept self-signed certs)");
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
