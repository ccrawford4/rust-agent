use std::fmt;

#[derive(Debug)]
pub enum KubeAgentError {
    HttpError(reqwest::Error),
    JsonParseError(serde_json::Error),
    ParseError(String),
}

impl fmt::Display for KubeAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KubeAgentError::HttpError(err) => write!(f, "HTTP request error: {}", err),
            KubeAgentError::JsonParseError(err) => write!(f, "JSON parsing error: {}", err),
            KubeAgentError::ParseError(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for KubeAgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KubeAgentError::HttpError(err) => Some(err),
            KubeAgentError::JsonParseError(err) => Some(err),
            KubeAgentError::ParseError(_) => None,
        }
    }
}

impl From<reqwest::Error> for KubeAgentError {
    fn from(err: reqwest::Error) -> Self {
        KubeAgentError::HttpError(err)
    }
}

impl From<serde_json::Error> for KubeAgentError {
    fn from(err: serde_json::Error) -> Self {
        KubeAgentError::JsonParseError(err)
    }
}
