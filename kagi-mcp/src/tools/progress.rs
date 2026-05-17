use rmcp::model::ProgressNotificationParam;
use rmcp::service::RequestContext;
use rmcp::RoleServer;

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
