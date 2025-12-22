use crate::agent::Agent;
use serde::{Deserialize, Serialize};
use std::io::{self, prelude::*};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
enum Method {
    GET,
    POST,
}

impl Method {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Path {
    Chat,
    Root,
}

impl Path {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "/chat" => Some(Path::Chat),
            "/" => Some(Path::Root),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Request {
    method: Method,
    path: Path,
    body: Option<String>,
}

impl Request {
    fn parse(request_str: &str) -> Option<Self> {
        let mut lines = request_str.lines();
        let first_line = lines.next()?;
        let mut parts = first_line.split_whitespace();

        let method = parts.next().and_then(Method::from_str)?;
        let path = parts.next().and_then(Path::from_str)?;

        // Parse headers to find body
        let mut content_length = 0;
        for line in lines.by_ref() {
            if line.is_empty() {
                break; // End of headers
            }
            if line.to_lowercase().starts_with("content-length:") {
                if let Some(len_str) = line.split(':').nth(1) {
                    content_length = len_str.trim().parse().unwrap_or(0);
                }
            }
        }

        // Read body if present
        let body = if content_length > 0 {
            let body_str: String = lines.collect::<Vec<_>>().join("\n");
            Some(body_str)
        } else {
            None
        };

        Some(Request { method, path, body })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub chat_history: Option<Vec<ChatMessage>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct Server {
    agent: Agent,
    host: String,
}

impl Server {
    pub fn new(agent: Agent, host: String) -> Self {
        Server { agent, host }
    }

    pub async fn listen(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.host)?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_client(stream).await {
                        eprintln!("Error handling client: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_client(&self, mut stream: TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = [0; 4096];
        let bytes_read = stream.read(&mut buffer)?;
        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        match Request::parse(&request_str) {
            Some(request) => {
                println!("Request: {:?}", request);

                match request.path {
                    Path::Chat => {
                        self.chat_handler(&mut stream, request.method, request.body)
                            .await
                    }
                    Path::Root => self.root_handler(&mut stream),
                }
            }
            None => Self::send_response(&mut stream, "400 Bad Request", "Invalid request"),
        }
    }

    fn send_response(stream: &mut TcpStream, status: &str, body: &str) -> io::Result<()> {
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
                        return Self::send_response(
                            stream,
                            "400 Bad Request",
                            "Missing request body",
                        )
                    }
                };

                match serde_json::from_str::<ChatRequest>(&body_str) {
                    Ok(chat_req) => {
                        println!("Received chat request - Prompt: {}", chat_req.prompt);
                        if let Some(history) = &chat_req.chat_history {
                            println!("Chat history length: {}", history.len());
                        }

                        let response = self.agent.prompt(chat_req.prompt).await;
                        Self::send_response(
                            stream,
                            "200 OK",
                            &format!("{{\"response\": \"{:?}\"}}", response),
                        )
                    }
                    Err(e) => {
                        eprintln!("Failed to parse chat request: {}", e);
                        Self::send_response(stream, "400 Bad Request", "Invalid JSON body")
                    }
                }
            }
            _ => Self::send_response(stream, "405 Method Not Allowed", "Invalid method for /chat"),
        }
    }

    fn root_handler(&self, stream: &mut TcpStream) -> io::Result<()> {
        Self::send_response(stream, "200 OK", "{\"healthy\": true}")
    }
}
