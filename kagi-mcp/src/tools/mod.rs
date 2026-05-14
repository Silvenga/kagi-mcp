pub mod extract;
pub mod search;

use kagi_api::error::KagiError;
use rmcp::model::{ErrorData, ProgressNotificationParam};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

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

pub(crate) fn map_kagi_error(error: KagiError) -> ErrorData {
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
pub(crate) async fn test_request_context() -> RequestContext<RoleServer> {
    use crate::server::KagiMcpServer;
    use rmcp::model::{ClientInfo, RequestId};
    use rmcp::service::serve_directly_with_ct;
    use std::sync::Arc;
    use tokio::io::duplex;
    use tokio_util::sync::CancellationToken;

    let (server_transport, client_transport) = duplex(4096);
    drop(client_transport);

    let server = KagiMcpServer::with_client(Arc::new(kagi_api::MockKagiApi::new()));
    let server_svc = serve_directly_with_ct(
        server,
        server_transport,
        None::<ClientInfo>,
        CancellationToken::new(),
    );

    let peer = server_svc.peer().clone();
    drop(server_svc);

    RequestContext::new(RequestId::Number(1), peer)
}
