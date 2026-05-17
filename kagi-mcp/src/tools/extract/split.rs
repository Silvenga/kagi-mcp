use crate::cache::{generate_cache_key, CacheStore};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::extract::errors::kagi_error_to_extract_error;
use crate::tools::extract::ExtractParams;
use crate::tools::progress::send_progress;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use std::sync::Arc;
use tokio::task::JoinSet;

pub async fn extract_split(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
    cache_store: Option<&CacheStore>,
) -> Result<CallToolResult, ErrorData> {
    let total_pages = pages.len();

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

    let mut results: Vec<Option<ExtractResponse>> = vec![None; total_pages];
    let mut pending: Vec<usize> = Vec::new();

    for (i, page) in pages.iter().enumerate() {
        let single_req = ExtractRequest::new(vec![page.clone()])
            .with_format("json".to_owned())
            .with_timeout_seconds(extract_timeout);

        let mut cache_hit = false;
        if params.cache {
            if let Some(store) = cache_store {
                let key = generate_cache_key(&single_req);
                if let Ok(Some(cached_bytes)) = store.get(&key).await {
                    if let Ok(cached_response) =
                        serde_json::from_slice::<ExtractResponse>(&cached_bytes)
                    {
                        let _ = send_progress(
                            ctx,
                            ((i + 1) as f64 / total_pages as f64) * 100.0,
                            Some(100.0),
                            format!("Page {}/{} (cached)", i + 1, total_pages),
                        )
                        .await;
                        results[i] = Some(cached_response);
                        cache_hit = true;
                    }
                }
            }
        }
        if !cache_hit {
            pending.push(i);
        }
    }

    let mut set = JoinSet::new();
    for &idx in &pending {
        if ctx.ct.is_cancelled() {
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }
        let client = Arc::clone(&client);
        let page = pages[idx].clone();
        let single_req = ExtractRequest::new(vec![page])
            .with_format("json".to_owned())
            .with_timeout_seconds(extract_timeout);

        set.spawn(async move {
            let result = client.extract(single_req).await;
            (idx, result)
        });
    }

    let mut collected = 0usize;
    while let Some(join_result) = set.join_next().await {
        if ctx.ct.is_cancelled() {
            set.abort_all();
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }

        match join_result {
            Ok((idx, Ok(api_response))) => {
                if params.cache {
                    if let Some(store) = cache_store {
                        let store_req = ExtractRequest::new(vec![pages[idx].clone()])
                            .with_format("json".to_owned())
                            .with_timeout_seconds(extract_timeout);
                        let key = generate_cache_key(&store_req);
                        if let Ok(json_bytes) = serde_json::to_vec(&api_response) {
                            let _ = store.set(&key, "extract", &json_bytes).await;
                        }
                    }
                }
                collected += 1;
                let _ = send_progress(
                    ctx,
                    ((total_pages - pending.len() + collected) as f64 / total_pages as f64) * 100.0,
                    Some(100.0),
                    format!("Page {}/{}", idx + 1, total_pages),
                )
                .await;
                results[idx] = Some(api_response);
            }
            Ok((idx, Err(kagi_err))) => {
                collected += 1;
                let _ = send_progress(
                    ctx,
                    ((total_pages - pending.len() + collected) as f64 / total_pages as f64) * 100.0,
                    Some(100.0),
                    format!("Page {}/{} (error)", idx + 1, total_pages),
                )
                .await;
                let extract_err = kagi_error_to_extract_error(&pages[idx].url, kagi_err);
                results[idx] = Some(ExtractResponse {
                    meta: kagi_api::Meta {
                        trace: String::new(),
                        node: None,
                        ms: None,
                    },
                    data: None,
                    errors: Some(vec![extract_err]),
                });
            }
            Err(_join_err) => {
                return Err(ErrorData::internal_error(
                    "Internal error: extract task panicked",
                    None,
                ));
            }
        }
    }

    let mut data: Vec<kagi_api::ExtractData> = Vec::new();
    let mut errors: Vec<kagi_api::ExtractError> = Vec::new();

    for result in results.into_iter().flatten() {
        if let Some(d) = result.data {
            data.extend(d);
        }
        if let Some(e) = result.errors {
            errors.extend(e);
        }
    }

    let response = ExtractResponse {
        meta: kagi_api::Meta {
            trace: String::new(),
            node: None,
            ms: None,
        },
        data: if data.is_empty() { None } else { Some(data) },
        errors: if errors.is_empty() {
            None
        } else {
            Some(errors)
        },
    };

    let _ = send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

    let content = if params.output_format == "json" {
        format_json(&response)
    } else {
        format_extract_markdown(&response)
    };
    let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
    Ok(CallToolResult::success(vec![Content::text(truncated)]))
}
