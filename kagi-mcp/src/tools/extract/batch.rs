use crate::cache::{generate_cache_key, CacheStore};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::errors::map_kagi_error;
use crate::tools::extract::errors::map_cache_error;
use crate::tools::extract::ExtractParams;
use crate::tools::progress::send_progress;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use std::sync::Arc;

pub async fn extract_batch(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
    cache_store: Option<&CacheStore>,
) -> Result<CallToolResult, ErrorData> {
    let request = ExtractRequest::new(pages)
        .with_format("json".to_owned())
        .with_timeout_seconds(extract_timeout);

    if params.cache {
        if let Some(store) = cache_store {
            let key = generate_cache_key(&request);
            match store.get(&key).await {
                Ok(Some(cached_bytes)) => {
                    let cached_response: ExtractResponse = serde_json::from_slice(&cached_bytes)
                        .map_err(|e| map_cache_error(e.into()))?;
                    let content = if params.output_format == "json" {
                        format_json(&cached_response)
                    } else {
                        format_extract_markdown(&cached_response)
                    };
                    let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
                    return Ok(CallToolResult::success(vec![Content::text(truncated)]));
                }
                Ok(None) => {}
                Err(e) => return Err(map_cache_error(e)),
            }
        }
    }

    let total_pages = params.pages.len();

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Extracting {total_pages} pages..."),
    )
    .await;

    if ctx.ct.is_cancelled() {
        return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
    }

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }
        result = client.extract(request.clone()) => result,
    };

    match result {
        Ok(response) => {
            let _ =
                send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

            if let Some(store) = cache_store {
                let key = generate_cache_key(&request);
                let json_bytes =
                    serde_json::to_vec(&response).map_err(|e| map_cache_error(e.into()))?;
                store
                    .set(&key, "extract", &json_bytes)
                    .await
                    .map_err(map_cache_error)?;
            }

            let content = if params.output_format == "json" {
                format_json(&response)
            } else {
                format_extract_markdown(&response)
            };
            let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Err(map_kagi_error(e)),
    }
}
