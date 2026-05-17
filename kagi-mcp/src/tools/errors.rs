use kagi_api::KagiError;
use rmcp::ErrorData;

pub fn map_kagi_error(error: KagiError) -> ErrorData {
    match error {
        KagiError::InvalidRequest { message } => {
            ErrorData::invalid_request(format!("Invalid request: {message}"), None)
        }
        KagiError::Unauthorized => {
            ErrorData::invalid_request("Unauthorized: Invalid Kagi API key", None)
        }
        KagiError::Forbidden => {
            ErrorData::invalid_request("Forbidden: IP address not authorized", None)
        }
        KagiError::RateLimited => {
            ErrorData::internal_error("Rate limited. Please retry later.", None)
        }
        KagiError::ServerError => {
            ErrorData::internal_error("Kagi API error. Please retry later.", None)
        }
        KagiError::Network { source } => {
            ErrorData::internal_error(format!("Request failed: {source}"), None)
        }
        KagiError::Api { status, message } => {
            ErrorData::internal_error(format!("Kagi API error (HTTP {status}): {message}"), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn when_invalid_request_then_should_map_to_invalid_request_error() {
        let err = KagiError::InvalidRequest {
            message: "bad param".into(),
        };
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INVALID_REQUEST);
        assert!(result.message.contains("Invalid request"));
        assert!(result.message.contains("bad param"));
    }

    #[test]
    fn when_unauthorized_then_should_map_to_invalid_request_error() {
        let err = KagiError::Unauthorized;
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INVALID_REQUEST);
        assert!(result.message.contains("Unauthorized"));
    }

    #[test]
    fn when_forbidden_then_should_map_to_invalid_request_error() {
        let err = KagiError::Forbidden;
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INVALID_REQUEST);
        assert!(result.message.contains("Forbidden"));
    }

    #[test]
    fn when_rate_limited_then_should_map_to_internal_error() {
        let err = KagiError::RateLimited;
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INTERNAL_ERROR);
        assert!(result.message.contains("Rate limited"));
    }

    #[test]
    fn when_server_error_then_should_map_to_internal_error() {
        let err = KagiError::ServerError;
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INTERNAL_ERROR);
        assert!(result.message.contains("Kagi API error"));
    }

    #[tokio::test]
    async fn when_network_error_then_should_map_to_internal_error() {
        use kagi_api::KagiClientBuilder;
        use kagi_api::SearchRequest;

        let client = KagiClientBuilder::new()
            .with_api_key("test")
            .with_base_url("http://127.0.0.1:1")
            .with_timeout_seconds(0.5)
            .build()
            .unwrap();
        let result = client.search(SearchRequest::new("test")).await;
        let kagi_err = result.unwrap_err();
        let result = map_kagi_error(kagi_err);
        assert_eq!(result.code, ErrorCode::INTERNAL_ERROR);
        assert!(result.message.contains("Request failed"));
    }

    #[test]
    fn when_api_error_then_should_map_to_internal_error() {
        let err = KagiError::Api {
            status: 418,
            message: "I'm a teapot".into(),
        };
        let result = map_kagi_error(err);
        assert_eq!(result.code, ErrorCode::INTERNAL_ERROR);
        assert!(result.message.contains("418"));
        assert!(result.message.contains("I'm a teapot"));
    }
}
