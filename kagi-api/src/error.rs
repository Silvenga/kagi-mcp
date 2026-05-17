use crate::types::KagiErrorResponse;
use reqwest::StatusCode;

/// Error type for Kagi API operations.
#[derive(Debug, thiserror::Error)]
pub enum KagiError {
    /// HTTP 401 - Invalid or missing API key.
    #[error("unauthorized: invalid or missing API key")]
    Unauthorized,

    /// HTTP 403 - API key lacks permission for the requested resource.
    #[error("forbidden: API key lacks permission")]
    Forbidden,

    /// HTTP 429 - Request rate limit exceeded.
    #[error("rate limited: too many requests")]
    RateLimited,

    /// HTTP 400 - Request parameters are invalid.
    #[error("invalid request: {message}")]
    InvalidRequest {
        /// The error message from the API.
        message: String,
    },

    /// HTTP 500 - Kagi server error.
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

impl KagiError {
    /// Maps an HTTP status code and optional error response body to a [`KagiError`].
    pub(crate) fn from_http_status(status: StatusCode, body: Option<KagiErrorResponse>) -> Self {
        let message = body
            .as_ref()
            .map(|b| b.format_message())
            .unwrap_or_default();

        match status {
            StatusCode::BAD_REQUEST => Self::InvalidRequest { message },
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited,
            StatusCode::INTERNAL_SERVER_ERROR => Self::ServerError,
            _ => Self::Api {
                status: status.as_u16(),
                message,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ErrorDetail;
    use reqwest::StatusCode;
    use std::time::Duration;

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
        let err = KagiError::from_http_status(StatusCode::BAD_REQUEST, Some(body));
        assert!(
            matches!(err, KagiError::InvalidRequest { .. }),
            "expected InvalidRequest, got {err}"
        );
        assert_eq!(err.to_string(), "invalid request: invalid parameter");
    }

    #[test]
    fn when_http_401_then_unauthorized_should_return() {
        let err = KagiError::from_http_status(StatusCode::UNAUTHORIZED, None);
        assert!(
            matches!(err, KagiError::Unauthorized),
            "expected Unauthorized, got {err}"
        );
    }

    #[test]
    fn when_http_403_then_forbidden_should_return() {
        let err = KagiError::from_http_status(StatusCode::FORBIDDEN, None);
        assert!(
            matches!(err, KagiError::Forbidden),
            "expected Forbidden, got {err}"
        );
    }

    #[test]
    fn when_http_429_then_rate_limited_should_return() {
        let err = KagiError::from_http_status(StatusCode::TOO_MANY_REQUESTS, None);
        assert!(
            matches!(err, KagiError::RateLimited),
            "expected RateLimited, got {err}"
        );
    }

    #[test]
    fn when_http_500_then_server_error_should_return() {
        let err = KagiError::from_http_status(StatusCode::INTERNAL_SERVER_ERROR, None);
        assert!(
            matches!(err, KagiError::ServerError),
            "expected ServerError, got {err}"
        );
    }

    #[test]
    fn when_http_418_then_api_variant_should_return() {
        let err = KagiError::from_http_status(StatusCode::IM_A_TEAPOT, None);
        assert!(
            matches!(err, KagiError::Api { status: 418, .. }),
            "expected Api(418), got {err}"
        );
    }

    #[tokio::test]
    async fn when_network_error_then_from_trait_should_map() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(100))
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
