use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use rmcp::schemars;
use serde::Deserialize;

use kagi_api::client::KagiClient;
use kagi_api::types::{ExtractPage, ExtractRequest};

use super::{map_kagi_error, send_progress};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    pub pages: Vec<String>,
    pub timeout: Option<f64>,
    pub output_format: Option<String>,
}

pub async fn extract_handler(
    client: &Arc<KagiClient>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let validated_urls = match crate::validation::validate_extract_urls(&params.pages) {
        Ok(urls) => urls,
        Err(e) => {
            return Err(rmcp::ErrorData::invalid_request(
                format!("URL validation failed: {e}"),
                None,
            ));
        }
    };

    let pages: Vec<ExtractPage> = validated_urls
        .into_iter()
        .map(|u| ExtractPage {
            url: u.to_string(),
        })
        .collect();

    let request = ExtractRequest {
        pages,
        timeout: params.timeout,
        format: Some("json".to_string()),
    };

    let total_pages = params.pages.len();

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Extracting {total_pages} pages..."),
    )
    .await;

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(rmcp::ErrorData::internal_error("Cancelled", None));
        }
        result = client.extract(request) => result,
    };

    for i in 1..=total_pages {
        let progress = (i as f64 / total_pages as f64) * 100.0;
        let _ = send_progress(
            ctx,
            progress,
            Some(100.0),
            format!("Extracted {i}/{total_pages} pages."),
        )
        .await;
    }

    let _ = send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_string()).await;

    match result {
        Ok(response) => {
            let output_format = params.output_format.as_deref().unwrap_or("markdown");
            let content = if output_format == "json" {
                crate::format::format_json(&response)
            } else {
                crate::format::format_extract_markdown(&response)
            };
            let truncated =
                crate::guard::truncate_response(&content, crate::guard::DEFAULT_MAX_RESPONSE_BYTES);
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Err(map_kagi_error(e)),
    }
}
