pub mod error;
pub mod tools;
pub mod types;

pub use error::KubeAgentError;
pub use tools::{ListNamespacesTool, ListPodsTool, NodeMetricsTool};

use tracing::*;

#[derive(Clone)]
pub struct KubeAgent {
    kube_api_server: String,
    token: String,
    certificate: Option<reqwest::Certificate>,
}

impl KubeAgent {
    pub fn new(
        kube_api_server: String,
        token: String,
        certificate: Option<reqwest::Certificate>,
    ) -> Self {
        KubeAgent {
            kube_api_server,
            token,
            certificate,
        }
    }

    pub async fn make_request(&self, endpoint: String) -> Result<String, KubeAgentError> {
        info!(
            "Connecting to Kubernetes API server at {}",
            self.kube_api_server
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );

        let client = if let Some(cert) = &self.certificate {
            reqwest::Client::builder()
                .default_headers(headers)
                .add_root_certificate(cert.clone())
                .build()
                .unwrap()
        } else {
            reqwest::Client::builder()
                .default_headers(headers)
                .danger_accept_invalid_certs(true) // Accept invalid certs for development
                // environments
                .build()
                .unwrap()
        };

        let request = client
            .get(format!("{}{}", self.kube_api_server, endpoint))
            .send()
            .await;

        match request {
            Ok(resp) => {
                let text = resp.text().await;
                match text {
                    Ok(body) => Ok(body),
                    Err(err) => {
                        error!("Error reading response body: {}", err);
                        Err(KubeAgentError::from(err))
                    }
                }
            }
            Err(err) => {
                error!("Error sending request to Kubernetes API server: {}", err);
                Err(KubeAgentError::from(err))
            }
        }
    }
}
