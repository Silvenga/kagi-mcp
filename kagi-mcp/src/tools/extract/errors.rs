pub fn kagi_error_to_extract_error(
    url: &str,
    error: kagi_api::KagiError,
) -> kagi_api::ExtractError {
    use kagi_api::KagiError;
    let (code, message) = match &error {
        KagiError::InvalidRequest { message: msg } => ("invalid_request", Some(msg.clone())),
        KagiError::Unauthorized => ("unauthorized", Some(error.to_string())),
        KagiError::Forbidden => ("forbidden", Some(error.to_string())),
        KagiError::RateLimited => ("rate_limited", Some(error.to_string())),
        KagiError::ServerError => ("server_error", Some(error.to_string())),
        KagiError::Network { source } => ("network_error", Some(source.to_string())),
        KagiError::Api {
            status,
            message: msg,
        } => ("api_error", Some(format!("HTTP {status}: {msg}"))),
    };
    kagi_api::ExtractError {
        url: url.to_owned(),
        code: code.to_owned(),
        message,
    }
}
