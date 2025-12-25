pub mod types;

use crate::agent::Agent;
use rig::completion::Message;
use std::io::{self, prelude::*};
use std::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};
use types::{ChatRequest, Method, Path, Request};

/// HTTP server that handles AI chat requests.
///
/// Implements a custom TCP-based HTTP/1.1 server without using a web framework.
/// Provides endpoints for health checks and AI-powered chat interactions.
pub struct Server {
    agent: Agent,
    host: String,
    api_key: String,
}

impl Server {
    pub fn new(agent: Agent, host: String, api_key: String) -> Self {
        Server {
            agent,
            host,
            api_key,
        }
    }

    /// Starts the server and listens for incoming connections.
    ///
    /// Blocks indefinitely, handling requests synchronously (one at a time).
    /// Each connection is processed completely before accepting the next one.
    pub async fn listen(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.host)?;
        info!("Server listening on {}", self.host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    debug!("Accepted connection from {:?}", stream.peer_addr());
                    if let Err(e) = self.handle_client(stream).await {
                        error!("Error handling client: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to accept connection: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handles a single client connection.
    ///
    /// Reads the HTTP request, validates the API key, routes to appropriate handler,
    /// and sends the response.
    async fn handle_client(&self, mut stream: TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = [0; 100000]; // 100KB buffer for request
        let bytes_read = stream.read(&mut buffer)?;
        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        match Request::parse(&request_str) {
            Some(request) => {
                debug!(
                    "Parsed request: method={:?}, path={:?}",
                    request.method, request.path
                );

                // Validate API key
                if let Some(api_key) = &request.api_key {
                    if *api_key != self.api_key {
                        warn!("Invalid API key attempt");
                        return Self::send_response(
                            &mut stream,
                            "403 Forbidden",
                            "Invalid API key",
                        );
                    }
                    debug!("API key validated successfully");
                } else {
                    warn!("Request missing API key");
                    return Self::send_response(&mut stream, "401 Unauthorized", "Missing API key");
                }

                match request.path {
                    Path::Chat => {
                        self.chat_handler(&mut stream, request.method, request.body)
                            .await
                    }
                    Path::Root => self.root_handler(&mut stream),
                    Path::Favicon => {
                        debug!("Favicon request received, returning 404");
                        Self::send_response(&mut stream, "404 Not Found", "Favicon not found")
                    }
                }
            }
            None => {
                warn!("Received malformed request, returning 400");
                debug!("Request string: {}", request_str);
                Self::send_response(&mut stream, "400 Bad Request", "Invalid request")
            }
        }
    }

    /// Sends an HTTP response to the client.
    fn send_response(stream: &mut TcpStream, status: &str, body: &str) -> io::Result<()> {
        debug!("Sending response: {}", status);
        let response = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()
    }

    /// Handles POST /chat requests by processing the prompt through the AI agent.
    async fn chat_handler(
        &self,
        stream: &mut TcpStream,
        method: Method,
        body: Option<String>,
    ) -> io::Result<()> {
        match method {
            Method::POST => {
                let body_str = match body {
                    Some(b) => b,
                    None => {
                        warn!("Chat request missing body");
                        return Self::send_response(
                            stream,
                            "400 Bad Request",
                            "Missing request body",
                        );
                    }
                };

                match serde_json::from_str::<ChatRequest>(&body_str) {
                    Ok(chat_req) => {
                        info!(
                            "Processing chat request ({} chars)",
                            chat_req.prompt.len()
                        );

                        // Convert chat history to internal message format
                        let mut chat_history: Vec<Message> = Vec::new();
                        if let Some(history) = chat_req.chat_history {
                            debug!("Including {} historical messages", history.len());
                            let mut converted_history = Vec::new();
                            for msg in history {
                                match msg.try_into() {
                                    Ok(m) => converted_history.push(m),
                                    Err(e) => {
                                        warn!("Invalid message role in chat history: {}", e);
                                        return Self::send_response(
                                            stream,
                                            "400 Bad Request",
                                            "Invalid message role in chat history",
                                        );
                                    }
                                }
                            }
                            chat_history = converted_history;
                        }

                        let response = self.agent.chat(chat_req.prompt, chat_history).await;
                        match response {
                            Ok(resp) => {
                                info!("Generated response ({} chars)", resp.len());
                                debug!("Response content: {}", resp);
                                Self::send_response(stream, "200 OK", &resp)
                            }
                            Err(e) => {
                                error!("Failed to generate chat response: {}", e);
                                Self::send_response(
                                    stream,
                                    "500 Internal Server Error",
                                    "Failed to generate response",
                                )
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse chat request JSON (request: {}), ERROR: {}",
                            &body_str, e
                        );
                        Self::send_response(stream, "400 Bad Request", "Invalid JSON body")
                    }
                }
            }
            _ => {
                warn!("Invalid HTTP method for /chat endpoint");
                Self::send_response(stream, "405 Method Not Allowed", "Invalid method for /chat")
            }
        }
    }

    /// Handles GET / requests (health check endpoint).
    fn root_handler(&self, stream: &mut TcpStream) -> io::Result<()> {
        debug!("Health check requested");
        Self::send_response(stream, "200 OK", "{\"healthy\": true}")
    }
}
