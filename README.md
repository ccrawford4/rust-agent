# AI Agent API Server

A RESTful API server built in Rust that provides AI-powered insights about my [portfolio website](https://about.calum.run) and its underlying Kubernetes infrastructure. 

## Features

- **AI-Powered Chat**: Natural language interface for querying portfolio information and infrastructure metrics
- **Kubernetes Integration**: Real-time access to pod listings, namespaces, and node metrics
- **Portfolio Scraping**: Fetches content from portfolio website sections (About, Work, Projects, Contact)
- **Secure Authentication**: API key-based request authentication

## Prerequisites

- **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs/)
- **OpenAI API Key**: Get one from [OpenAI](https://platform.openai.com/)
- **Kubernetes Cluster** (optional): Required for infrastructure monitoring features

## Quick Start

### Local Development

1. **Clone the repository**
   ```bash
   git clone https://github.com/ccrawford4/rust-agent.git
   cd rust-agent
   ```

2. **Create a secure key to access the chat API**
   ```bash
   openssl rand -base64 32
   ```

3. **Create a `.env` file** in the project root:
   ```env
   # Required
   OPENAI_API_KEY=your_openai_api_key_here
   CHAT_API_KEY=your_secure_api_key_for_authentication_from_step_2

   # Optional (defaults shown)
   PRODUCTION_MODE=false
   KUBE_API_SERVER=https://localhost:6443
   KUBE_TOKEN=your_kubernetes_token_here
   RUST_LOG=info
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run --release
   ```

5. **Test the server**
   ```bash
   # Health check
   curl -H "X-API-Key: your_secure_api_key_for_authentication" \
     http://127.0.0.1:8080/

   # Chat request
   curl -X POST http://127.0.0.1:8080/chat \
     -H "Content-Type: application/json" \
     -H "X-API-Key: your_secure_api_key_for_authentication" \
     -d '{
       "prompt": "What is on Calum'\''s About page?",
       "chat_history": []
     }'
   ```

### Production Deployment (Kubernetes)

The server is designed to run inside a Kubernetes cluster as a pod with appropriate RBAC permissions.

1. **Build Docker image**
   ```bash
   docker build -t ai-agent-api:latest .
   ```

2. **Apply Kubernetes manifests**
   ```bash
   # Apply RBAC permissions
   kubectl apply -f kubernetes/permissions.yaml

   # Deploy the application
   kubectl apply -f kubernetes/agent-deployment.yaml
   ```

3. **Configure environment**

   The production deployment uses:
   - **Service Account Tokens**: Automatically mounted at `/var/run/secrets/kubernetes.io/serviceaccount/token`
   - **CA Certificates**: Mounted at `/var/run/secrets/kubernetes.io/serviceaccount/ca.crt`
   - **Environment Variables**: Set via ConfigMap/Secret

   Required environment variables for production:
   ```yaml
   env:
     - name: PRODUCTION_MODE
       value: "true"
     - name: OPENAI_API_KEY
       valueFrom:
         secretKeyRef:
           name: ai-agent-secrets
           key: openai-api-key
     - name: CHAT_API_KEY
       valueFrom:
         secretKeyRef:
           name: ai-agent-secrets
           key: chat-api-key
     - name: KUBE_API_SERVER
       value: "https://kubernetes.default.svc"
   ```

## API Documentation

### Endpoints

#### `GET /`
Health check endpoint.

**Response**
```json
{
  "healthy": true
}
```

#### `POST /chat`
Main chat endpoint for AI interactions.

**Request Headers**
- `Content-Type: application/json`
- `X-API-Key: <your-api-key>`

**Request Body**
```json
{
  "prompt": "Your question here",
  "chat_history": [
    {
      "role": "user",
      "content": "Previous user message"
    },
    {
      "role": "assistant",
      "content": "Previous assistant response"
    }
  ]
}
```

**Response**
```
String response from the AI agent
```

**Status Codes**
- `200 OK`: Successful response
- `400 Bad Request`: Invalid JSON or malformed request
- `401 Unauthorized`: Missing API key
- `403 Forbidden`: Invalid API key
- `405 Method Not Allowed`: Wrong HTTP method
- `500 Internal Server Error`: AI agent failure

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `OPENAI_API_KEY` | Yes | - | OpenAI API key for GPT-5.1 model |
| `CHAT_API_KEY` | Yes | - | API key for authenticating requests to this server |
| `PRODUCTION_MODE` | No | `false` | Enables production mode (uses mounted K8s credentials) |
| `KUBE_API_SERVER` | No | `https://localhost:6443` | Kubernetes API server URL |
| `KUBE_TOKEN` | No (dev only) | - | Kubernetes bearer token (development mode only) |
| `RUST_LOG` | No | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |

### Logging

The server uses structured logging with the `tracing` framework. Control verbosity with the `RUST_LOG` environment variable:

```bash
# Show only errors and warnings
RUST_LOG=warn cargo run

# Show detailed debug information
RUST_LOG=debug cargo run

# Show everything including trace logs
RUST_LOG=trace cargo run

# Filter by module
RUST_LOG=sql_agent::agent=debug,sql_agent::kube=trace cargo run
```

**Log Levels by Component**
- `error`: Critical failures (server startup, OpenAI client errors, K8s connection failures)
- `warn`: Non-critical issues (missing API keys, invalid requests, self-signed certificates)
- `info`: Important events (server start, successful requests, tool invocations)
- `debug`: Detailed flow (request parsing, API responses, data transformations)

## Architecture

### Project Structure

```
src/
├── main.rs              # Application entry point
├── environment.rs       # Configuration management
├── server/              # HTTP server implementation
│   ├── mod.rs          # TCP-based HTTP/1.1 server
│   └── types.rs        # Request/Response types
├── agent/               # AI agent module
│   ├── mod.rs          # Agent initialization and chat handler
│   └── tools/          # Portfolio scraping tools
│       ├── mod.rs
│       └── web_search.rs
└── kube/                # Kubernetes integration
    ├── mod.rs          # KubeAgent HTTP client
    ├── error.rs        # Custom error types
    ├── types/          # Kubernetes API response types
    │   ├── mod.rs
    │   ├── pod.rs
    │   ├── metrics.rs
    │   └── namespaces.rs
    └── tools/          # Kubernetes tools for AI agent
        ├── mod.rs
        ├── pods.rs     # ListPodsTool
        ├── namespaces.rs # ListNamespacesTool
        └── metrics.rs  # NodeMetricsTool
```

### How It Works

1. **Server** receives HTTP requests on port 8080
2. **Request parsing** extracts method, path, API key, and body
3. **Authentication** validates the API key from the `X-API-Key` header
4. **Routing** directs to appropriate handler (`/` or `/chat`)
5. **AI Agent** processes the chat request:
   - Receives user prompt and chat history
   - Decides which tools to invoke (web scraping, Kubernetes queries)
   - Makes up to 2 rounds of tool calls
   - Generates natural language response
6. **Response** is sent back to client

### Tools Available to AI Agent

1. **WebSearch**: Fetches content from portfolio sections
   - Supports: About, Work, Projects, Contact pages
   - Environment-aware (production vs local portfolio URLs)

2. **ProfileUrlList**: Lists available portfolio URLs

3. **ListPodsTool**: Queries Kubernetes pods
   - Optional namespace filtering
   - Configurable result limit

4. **ListNamespacesTool**: Lists all cluster namespaces

5. **NodeMetricsTool**: Gets node CPU and memory metrics
   - Requires metrics-server addon
   - Calculates usage percentages
   - Fetches data from both core API and metrics API in parallel

## Development

### Running Tests
```bash
cargo test
```

### Building for Release
```bash
cargo build --release
# Binary located at: target/release/sql-agent
```

### Code Formatting
```bash
cargo fmt
```

### Linting
```bash
cargo clippy
```

## Security Considerations

- **API Key Authentication**: All requests must include a valid `X-API-Key` header
- **Certificate Validation**: Production mode uses CA certificates for secure K8s communication
- **Development Mode**: Accepts self-signed certificates (never use in production)
- **Secrets Management**: Use Kubernetes Secrets for sensitive environment variables
- **RBAC Permissions**: Ensure the service account has minimal required permissions

## Troubleshooting

### Server won't start
- Check that port 8080 is not already in use
- Verify `OPENAI_API_KEY` is set correctly
- Check logs with `RUST_LOG=debug` for detailed error messages

### Kubernetes connection failed
- Verify `KUBE_API_SERVER` URL is correct
- Ensure `KUBE_TOKEN` is valid (development mode)
- Check that service account has proper RBAC permissions (production mode)
- Confirm metrics-server is installed for node metrics

### AI agent errors
- Verify OpenAI API key is valid and has credits
- Check network connectivity to OpenAI API
- Review error logs for specific OpenAI error messages

### 401/403 responses
- Ensure `X-API-Key` header is included in request
- Verify the API key matches `CHAT_API_KEY` environment variable
