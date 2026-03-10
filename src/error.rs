use std::fmt;
use std::time::Duration;

#[derive(Debug)]
#[allow(dead_code)]
pub enum CapxError {
    Auth {
        message: String,
    },
    Api {
        status: u16,
        message: String,
        endpoint: String,
    },
    NotFound {
        id: String,
    },
    RateLimited {
        retry_after: Option<Duration>,
    },
    Parse {
        context: String,
    },
    Network {
        source: String,
    },
}

impl fmt::Display for CapxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapxError::Auth { message } => write!(f, "Authentication failed: {message}"),
            CapxError::Api {
                status,
                message,
                endpoint,
            } => {
                write!(f, "API error {status} on {endpoint}: {message}")
            }
            CapxError::NotFound { id } => write!(f, "Not found: {id}"),
            CapxError::RateLimited { retry_after } => {
                if let Some(d) = retry_after {
                    write!(f, "Rate limited. Retry after {d:?}")
                } else {
                    write!(f, "Rate limited. Please try again later.")
                }
            }
            CapxError::Parse { context } => write!(f, "Parse error: {context}"),
            CapxError::Network { source } => write!(f, "Network error: {source}"),
        }
    }
}

impl std::error::Error for CapxError {}

#[allow(dead_code)]
impl CapxError {
    pub fn error_type(&self) -> &'static str {
        match self {
            CapxError::Auth { .. } => "auth",
            CapxError::Api { .. } => "api",
            CapxError::NotFound { .. } => "not_found",
            CapxError::RateLimited { .. } => "rate_limited",
            CapxError::Parse { .. } => "parse",
            CapxError::Network { .. } => "network",
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "type": self.error_type(),
                "message": self.to_string(),
            }
        })
    }
}
