use std::io::Write;
use std::net::TcpStream;
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

    pub fn get_pods(
        &self,
        namespace: Option<String>,
        limit: Option<u32>,
    ) -> Result<String, std::io::Error> {
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

        self.make_request(endpoint)
    }

    fn make_request(&self, endpoint: String) -> Result<String, std::io::Error> {
        // Connect to the kube api server
        info!(
            "Connecting to Kubernetes API server at {}",
            self.kube_api_server
        );

        if let Err(err) = TcpStream::connect(self.kube_api_server.clone()) {
            error!("Failed to connect to Kubernetes API server: {}", err);
            return Err(err);
        }

        let mut stream = TcpStream::connect(self.kube_api_server.clone()).unwrap();

        // Make the GET request with the token in the header
        let request = format!(
            "GET {} HTTP/1.1\r\n\
         User-Agent: Rust-Client/1.0\r\n\
         Authorization: Bearer {}\r\n\
         \r\n", // The crucial blank line that ends the header section
            endpoint, self.token
        );

        info!(
            "Making request to endpoint: {}. (Request: {})",
            endpoint, request
        );

        // If we fail to write, return the error
        if let Err(err) = stream.write(request.as_bytes()) {
            error!("Failed to write to stream: {}", err);
            return Err(err);
        } else {
            let mut response = String::new();
            if let Err(err) = std::io::Read::read_to_string(&mut stream, &mut response) {
                error!("Failed to read from stream: {}", err);
                return Err(err);
            } else {
                return Ok(response);
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
