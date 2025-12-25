use rig::completion::Message;
use serde::{Deserialize, Serialize};

/// HTTP methods supported by the server
#[derive(Debug)]
pub enum Method {
    GET,
    POST,
}

impl Method {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            _ => None,
        }
    }
}

/// HTTP paths (routes) supported by the server
#[derive(Debug)]
pub enum Path {
    /// POST /chat - Main chat endpoint for AI interactions
    Chat,
    /// GET / - Health check endpoint
    Root,
    /// GET /favicon.ico - Favicon request (returns 404)
    Favicon,
}

impl Path {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "/chat" => Some(Path::Chat),
            "/" => Some(Path::Root),
            "/favicon.ico" => Some(Path::Favicon),
            _ => None,
        }
    }
}

/// Parsed HTTP request with relevant fields extracted
#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: Path,
    pub api_key: Option<String>,
    pub body: Option<String>,
}

impl Request {
    /// Parses an HTTP/1.1 request string into a Request struct.
    ///
    /// Extracts:
    /// - HTTP method and path from the request line
    /// - X-API-Key header for authentication
    /// - Request body based on Content-Length header
    ///
    /// Returns None if the request is malformed or uses unsupported method/path.
    pub fn parse(request_str: &str) -> Option<Self> {
        let mut lines = request_str.lines();
        let first_line = lines.next()?;
        let mut parts = first_line.split_whitespace();

        let method = parts.next().and_then(Method::from_str)?;
        let path = parts.next().and_then(Path::from_str)?;

        let mut content_length = 0;
        let mut api_key = None;

        // Parse headers
        for line in lines.by_ref() {
            if line.is_empty() {
                break;
            }
            if line.to_lowercase().starts_with("x-api-key:") {
                if let Some(key_str) = line.split(':').nth(1) {
                    api_key = Some(key_str.trim().to_string());
                }
            }
            if line.to_lowercase().starts_with("content-length:") {
                if let Some(len_str) = line.split(':').nth(1) {
                    content_length = len_str.trim().parse().unwrap_or(0);
                }
            }
        }

        // Extract body if present
        let body = if content_length > 0 {
            let body_str: String = lines.collect::<Vec<_>>().join("\n");
            Some(body_str)
        } else {
            None
        };

        Some(Request {
            method,
            path,
            body,
            api_key,
        })
    }
}

/// Request payload for the /chat endpoint
#[derive(Debug, Deserialize, Serialize)]
pub struct ChatRequest {
    /// The user's prompt/question
    pub prompt: String,
    /// Optional conversation history for context
    pub chat_history: Option<Vec<HttpMessage>>,
}

/// A single message in a chat conversation
#[derive(Debug, Deserialize, Serialize)]
pub struct HttpMessage {
    /// Message role: "user" or "assistant"
    pub role: String,
    /// Message content/text
    pub content: String,
}

impl TryFrom<HttpMessage> for Message {
    type Error = &'static str;

    fn try_from(value: HttpMessage) -> Result<Self, Self::Error> {
        match value.role.as_str() {
            "user" => Ok(Message::user(value.content)),
            "assistant" => Ok(Message::assistant(value.content)),
            _ => Err("Invalid role in HttpMessage"),
        }
    }
}
