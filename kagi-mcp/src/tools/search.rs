use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use rmcp::schemars;
use serde::Deserialize;

use kagi_api::client::KagiClient;
use kagi_api::types::{Filters, SearchRequest};

use super::{map_kagi_error, send_progress};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    pub query: String,
    pub workflow: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub output_format: Option<String>,
}

pub async fn search_handler(
    client: &Arc<KagiClient>,
    params: SearchParams,
    ctx: &RequestContext<RoleServer>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let request = SearchRequest {
        query: params.query.clone(),
        workflow: params.workflow.clone(),
        format: Some("json".to_string()),
        timeout: None,
        page: None,
        limit: None,
        safe_search: None,
        region: None,
        filters: build_filters(params.after, params.before),
    };

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Searching \"{}\"", params.query),
    )
    .await;

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(rmcp::ErrorData::internal_error("Cancelled", None));
        }
        result = client.search(request) => result,
    };

    let _ = send_progress(ctx, 100.0, Some(100.0), "Query completed.".to_string()).await;

    match result {
        Ok(response) => {
            let output_format = params.output_format.as_deref().unwrap_or("markdown");
            let content = if output_format == "json" {
                crate::format::format_json(&response)
            } else {
                crate::format::format_search_markdown(&response)
            };
            let truncated =
                crate::guard::truncate_response(&content, crate::guard::DEFAULT_MAX_RESPONSE_BYTES);
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Err(map_kagi_error(e)),
    }
}

fn build_filters(after: Option<String>, before: Option<String>) -> Option<Filters> {
    if after.is_some() || before.is_some() {
        Some(Filters {
            after,
            before,
            region: None,
        })
    } else {
        None
    }
}
