pub mod types;

use crate::agent::Agent;
use rig::completion::Message;
use std::io::{self, prelude::*};
use std::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};
use types::{ChatRequest, Method, Path, Request};

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

    pub async fn listen(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.host)?;
        info!("Server listening on {}", self.host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    debug!("Accepted new connection from {:?}", stream.peer_addr());
                    if let Err(e) = self.handle_client(stream).await {
                        error!("Error handling client: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_client(&self, mut stream: TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = [0; 100000];
        let bytes_read = stream.read(&mut buffer)?;
        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        match Request::parse(&request_str) {
            Some(request) => {
                debug!(
                    "Parsed request: method={:?}, path={:?}",
                    request.method, request.path
                );

                if let Some(api_key) = &request.api_key {
                    debug!("API key provided: {}", api_key);
                    if *api_key != self.api_key {
                        warn!("Invalid API key provided");
                        return Self::send_response(
                            &mut stream,
                            "403 Forbidden",
                            "Invalid API key",
                        );
                    }
                } else {
                    warn!("No API key provided in request");
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

    fn send_response(stream: &mut TcpStream, status: &str, body: &str) -> io::Result<()> {
        debug!("Sending response: status={}", status);
        let response = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()
    }

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
                            "Received chat request - prompt length: {} chars",
                            chat_req.prompt.len()
                        );
                        let mut chat_history: Vec<Message> = Vec::new();
                        if let Some(history) = chat_req.chat_history {
                            debug!("Chat history length: {} messages", history.len());
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
                                info!("Successfully generated chat response: {}", resp);
                                Self::send_response(stream, "200 OK", &resp)
                            }
                            Err(e) => {
                                error!("Error generating chat response: {}", e);
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

    fn root_handler(&self, stream: &mut TcpStream) -> io::Result<()> {
        debug!("Health check request received");
        Self::send_response(stream, "200 OK", "{\"healthy\": true}")
    }
}
