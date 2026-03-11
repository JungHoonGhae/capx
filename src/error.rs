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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn auth_error_display() {
        let e = CapxError::Auth {
            message: "bad token".to_string(),
        };
        assert_eq!(e.to_string(), "Authentication failed: bad token");
    }

    #[test]
    fn api_error_display() {
        let e = CapxError::Api {
            status: 404,
            message: "not found".to_string(),
            endpoint: "/spaces".to_string(),
        };
        assert_eq!(e.to_string(), "API error 404 on /spaces: not found");
    }

    #[test]
    fn not_found_display() {
        let e = CapxError::NotFound {
            id: "abc-123".to_string(),
        };
        assert_eq!(e.to_string(), "Not found: abc-123");
    }

    #[test]
    fn rate_limited_with_retry() {
        let e = CapxError::RateLimited {
            retry_after: Some(Duration::from_secs(5)),
        };
        let s = e.to_string();
        assert!(s.contains("Rate limited"));
        assert!(s.contains("5s"));
    }

    #[test]
    fn rate_limited_no_retry() {
        let e = CapxError::RateLimited { retry_after: None };
        assert_eq!(e.to_string(), "Rate limited. Please try again later.");
    }

    #[test]
    fn parse_error_display() {
        let e = CapxError::Parse {
            context: "invalid JSON".to_string(),
        };
        assert_eq!(e.to_string(), "Parse error: invalid JSON");
    }

    #[test]
    fn network_error_display() {
        let e = CapxError::Network {
            source: "connection refused".to_string(),
        };
        assert_eq!(e.to_string(), "Network error: connection refused");
    }

    #[test]
    fn error_type_returns_correct_string() {
        assert_eq!(
            CapxError::Auth {
                message: "x".to_string()
            }
            .error_type(),
            "auth"
        );
        assert_eq!(
            CapxError::Api {
                status: 500,
                message: "x".to_string(),
                endpoint: "/x".to_string()
            }
            .error_type(),
            "api"
        );
        assert_eq!(
            CapxError::NotFound {
                id: "x".to_string()
            }
            .error_type(),
            "not_found"
        );
        assert_eq!(
            CapxError::RateLimited { retry_after: None }.error_type(),
            "rate_limited"
        );
        assert_eq!(
            CapxError::Parse {
                context: "x".to_string()
            }
            .error_type(),
            "parse"
        );
        assert_eq!(
            CapxError::Network {
                source: "x".to_string()
            }
            .error_type(),
            "network"
        );
    }

    #[test]
    fn to_json_has_error_type_and_message() {
        let e = CapxError::Auth {
            message: "bad token".to_string(),
        };
        let json = e.to_json();
        assert_eq!(json["error"]["type"], "auth");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bad token"));
    }

    #[test]
    fn to_json_api_error() {
        let e = CapxError::Api {
            status: 403,
            message: "forbidden".to_string(),
            endpoint: "/spaces".to_string(),
        };
        let json = e.to_json();
        assert_eq!(json["error"]["type"], "api");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("403"));
    }
}

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
