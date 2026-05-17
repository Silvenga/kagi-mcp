use kagi_api::KagiError;
use rmcp::model::{ErrorData, ProgressNotificationParam};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

pub fn default_true() -> bool {
    true
}

pub fn default_markdown() -> String {
    "markdown".to_owned()
}

pub async fn send_progress(
    ctx: &RequestContext<RoleServer>,
    progress: f64,
    total: Option<f64>,
    message: String,
) {
    if let Some(token) = ctx.meta.get_progress_token() {
        let mut param = ProgressNotificationParam::new(token, progress);
        if let Some(total) = total {
            param = param.with_total(total);
        }
        param = param.with_message(message);
        let _ = ctx.peer.notify_progress(param).await;
    }
}

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
