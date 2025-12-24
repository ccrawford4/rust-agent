pub struct KubeAgent {
    kube_api_server: String,
}

impl KubeAgent {
    fn new(kube_api_server: String) -> Self {
        return KubeAgent { kube_api_server };
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
