use tracing::*;

pub struct KubeAgent {
    kube_api_server: String,
    token: String,
}

impl KubeAgent {
    pub fn new(kube_api_server: String, token: String) -> Self {
        return KubeAgent {
            kube_api_server,
            token,
        };
    }

    pub async fn get_pods(
        &self,
        namespace: Option<String>,
        limit: Option<u32>,
    ) -> Result<String, reqwest::Error> {
        // https://localhost:50220/api/v1/namespaces/default/pods?limit=500
        let mut namespace_path = String::from("default");
        let mut limit_query: u32 = 500;

        // Override defaults if provided
        if let Some(ns) = namespace {
            namespace_path = ns;
        }
        if let Some(lim) = limit {
            limit_query = lim;
        }

        // Construct endpoint
        let endpoint = format!(
            "/api/v1/namespaces/{}/pods?limit={}",
            namespace_path, limit_query
        );

        self.make_request(endpoint).await
    }

    async fn make_request(&self, endpoint: String) -> Result<String, reqwest::Error> {
        // Connect to the kube api server
        info!(
            "Connecting to Kubernetes API server at {}",
            self.kube_api_server
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .danger_accept_invalid_certs(true) // Accept invalid certs for local clusters
            .build()
            .unwrap();

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
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!("Error sending request to Kubernetes API server: {}", err);
                Err(err)
            }
        }
    }
}

// TODO:
// 1. Connect to minikube and setup a service account and token for the service account to
//    authenticate
//
//    Public Agent:
//
// 2. Update the env + this module to use the token so we can connect to the k8s api server
//    directly (k3s also uses token auth so this method should work for both!)
//
//
//  Some operations we will want (very locked down)
//  - Pods (list the pods and their names)
//  - Cluster info (memory, cpu, etc)
//  - Namespaces (how many, names)
//
//  NO Logs, service accounts, or any other info will be exposed to the agent!
//
//
//  Private agent (long term):
//  - Setup a private agent that can actually modify pods/deployments/etc
//  - Create new deployments
//  - Output the results as yaml/json
//  - Cut a PR to introduce the change!
