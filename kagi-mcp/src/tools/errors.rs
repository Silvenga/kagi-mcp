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
