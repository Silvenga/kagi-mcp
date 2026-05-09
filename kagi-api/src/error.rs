use reqwest::StatusCode;
use serde::Deserialize;

/// Error type for Kagi API operations.
#[derive(Debug, thiserror::Error)]
pub enum KagiError {
    /// HTTP 401 — Invalid or missing API key.
    #[error("unauthorized: invalid or missing API key")]
    Unauthorized,

    /// HTTP 403 — API key lacks permission for the requested resource.
    #[error("forbidden: API key lacks permission")]
    Forbidden,

    /// HTTP 429 — Request rate limit exceeded.
    #[error("rate limited: too many requests")]
    RateLimited,

    /// HTTP 400 — Request parameters are invalid.
    #[error("invalid request: {message}")]
    InvalidRequest {
        /// The error message from the API.
        message: String,
    },

    /// HTTP 500 — Kagi server error.
    #[error("server error")]
    ServerError,

    /// Network or timeout error from the HTTP client.
    #[error("network error: {source}")]
    Network {
        /// The underlying reqwest error.
        #[from]
        source: reqwest::Error,
    },

    /// Catch-all for unexpected HTTP status codes.
    #[error("API error (HTTP {status}): {message}")]
    Api {
        /// The HTTP status code.
        status: u16,
        /// The error message from the API.
        message: String,
    },
}

/// Error detail as returned by the Kagi API.
#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[expect(dead_code)]
    code: String,
    #[expect(dead_code)]
    url: String,
    message: Option<String>,
    #[expect(dead_code)]
    location: Option<String>,
}

/// The Kagi API error response envelope.
///
/// Returned when a request fails. The `error` array contains one or more
/// error detail objects describing what went wrong.
#[derive(Debug, Deserialize)]
pub struct KagiErrorResponse {
    #[expect(dead_code)]
    meta: serde_json::Value,
    #[expect(dead_code)]
    data: Option<serde_json::Value>,
    error: Vec<ErrorDetail>,
}

impl KagiErrorResponse {
    /// Formats the error details into a human-readable message.
    fn format_message(&self) -> String {
        self.error
            .iter()
            .filter_map(|e| e.message.as_deref())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Maps an HTTP status code and optional error response body to a [`KagiError`].
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn from_http_status(
    status: StatusCode,
    body: Option<KagiErrorResponse>,
) -> KagiError {
    let message = body
        .as_ref()
        .map(|b| b.format_message())
        .unwrap_or_default();

    match status {
        StatusCode::BAD_REQUEST => KagiError::InvalidRequest { message },
        StatusCode::UNAUTHORIZED => KagiError::Unauthorized,
        StatusCode::FORBIDDEN => KagiError::Forbidden,
        StatusCode::TOO_MANY_REQUESTS => KagiError::RateLimited,
        StatusCode::INTERNAL_SERVER_ERROR => KagiError::ServerError,
        _ => KagiError::Api {
            status: status.as_u16(),
            message,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    fn make_error_response(message: Option<&str>) -> KagiErrorResponse {
        KagiErrorResponse {
            meta: serde_json::Value::Object(Default::default()),
            data: None,
            error: vec![ErrorDetail {
                code: String::new(),
                url: String::new(),
                message: message.map(String::from),
                location: None,
            }],
        }
    }

    #[test]
    fn when_http_400_then_invalid_request_should_return() {
        let body = make_error_response(Some("invalid parameter"));
        let err = from_http_status(StatusCode::BAD_REQUEST, Some(body));
        assert!(
            matches!(err, KagiError::InvalidRequest { .. }),
            "expected InvalidRequest, got {err}"
        );
        assert_eq!(err.to_string(), "invalid request: invalid parameter");
    }

    #[test]
    fn when_http_401_then_unauthorized_should_return() {
        let err = from_http_status(StatusCode::UNAUTHORIZED, None);
        assert!(
            matches!(err, KagiError::Unauthorized),
            "expected Unauthorized, got {err}"
        );
    }

    #[test]
    fn when_http_403_then_forbidden_should_return() {
        let err = from_http_status(StatusCode::FORBIDDEN, None);
        assert!(
            matches!(err, KagiError::Forbidden),
            "expected Forbidden, got {err}"
        );
    }

    #[test]
    fn when_http_429_then_rate_limited_should_return() {
        let err = from_http_status(StatusCode::TOO_MANY_REQUESTS, None);
        assert!(
            matches!(err, KagiError::RateLimited),
            "expected RateLimited, got {err}"
        );
    }

    #[test]
    fn when_http_500_then_server_error_should_return() {
        let err = from_http_status(StatusCode::INTERNAL_SERVER_ERROR, None);
        assert!(
            matches!(err, KagiError::ServerError),
            "expected ServerError, got {err}"
        );
    }

    #[test]
    fn when_http_418_then_api_variant_should_return() {
        let err = from_http_status(StatusCode::IM_A_TEAPOT, None);
        assert!(
            matches!(err, KagiError::Api { status: 418, .. }),
            "expected Api(418), got {err}"
        );
    }

    #[tokio::test]
    async fn when_network_error_then_from_trait_should_map() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        let result = client.get("http://127.0.0.1:1").send().await;
        let err: KagiError = result.unwrap_err().into();
        assert!(
            matches!(err, KagiError::Network { .. }),
            "expected Network, got {err}"
        );
    }
}
