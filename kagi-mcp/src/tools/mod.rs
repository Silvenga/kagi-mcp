pub mod extract;
pub mod search;

use rmcp::model::ProgressNotificationParam;
use rmcp::service::RequestContext;
use rmcp::RoleServer;

use kagi_api::error::KagiError;

pub(crate) async fn send_progress(
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

pub(crate) fn map_kagi_error(error: KagiError) -> rmcp::ErrorData {
    match error {
        KagiError::InvalidRequest { message } => {
            rmcp::ErrorData::internal_error(format!("Invalid request: {message}"), None)
        }
        KagiError::Unauthorized => {
            rmcp::ErrorData::internal_error("Unauthorized: Invalid Kagi API key", None)
        }
        KagiError::Forbidden => {
            rmcp::ErrorData::internal_error("Forbidden: IP address not authorized", None)
        }
        KagiError::RateLimited => {
            rmcp::ErrorData::internal_error("Rate limited. Please retry later.", None)
        }
        KagiError::ServerError => {
            rmcp::ErrorData::internal_error("Kagi API error. Please retry later.", None)
        }
        KagiError::Network { source } => {
            rmcp::ErrorData::internal_error(format!("Request failed: {source}"), None)
        }
        KagiError::Api { status, message } => {
            rmcp::ErrorData::internal_error(
                format!("Kagi API error (HTTP {status}): {message}"),
                None,
            )
        }
    }
}
