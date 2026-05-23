use crate::cache::{generate_cache_key, CacheStore};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::errors::map_kagi_error;
use crate::tools::extract::errors::map_cache_error;
use crate::tools::extract::fallback::{is_empty_content, FallbackMatch, FallbackRules};
use crate::tools::extract::ExtractParams;
use crate::tools::progress::send_progress;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractData, ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn extract_batch(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
    cache_store: Option<&CacheStore>,
    fallback_rules: Option<&FallbackRules>,
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

    let mut synthetic_data: Vec<(usize, ExtractData)> = Vec::new();
    let mut api_pages: Vec<ExtractPage> = Vec::new();
    let original_pages = pages.clone();

    if let Some(rules) = fallback_rules {
        let (blocked, unblocked) = rules.filter_urls(&pages);
        let mut blocked_map: HashMap<usize, FallbackMatch> = blocked.into_iter().collect();
        let mut unblocked_map: HashMap<usize, ExtractPage> = unblocked.into_iter().collect();

        for (index, page) in pages.iter().enumerate() {
            if let Some(matched) = blocked_map.remove(&index) {
                match matched {
                    FallbackMatch::AlwaysBlock { message } => {
                        synthetic_data.push((
                            index,
                            ExtractData {
                                url: page.url.clone(),
                                markdown: Some(message),
                            },
                        ));
                    }
                    FallbackMatch::EmptyContent { .. } => {
                        api_pages.push(page.clone());
                    }
                    FallbackMatch::NoMatch => {}
                }
            } else if let Some(page) = unblocked_map.remove(&index) {
                api_pages.push(page);
            } else {
                // Index was neither blocked nor unblocked — no action needed.
            }
        }
    } else {
        api_pages = pages;
    }

    if api_pages.is_empty() {
        let _ = send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

        let response = ExtractResponse {
            meta: kagi_api::Meta {
                trace: String::new(),
                node: None,
                ms: None,
            },
            data: if synthetic_data.is_empty() {
                None
            } else {
                let mut merged = synthetic_data;
                merged.sort_by_key(|(index, _)| *index);
                Some(merged.into_iter().map(|(_, data)| data).collect())
            },
            errors: None,
        };

        let content = if params.output_format == "json" {
            format_json(&response)
        } else {
            format_extract_markdown(&response)
        };
        let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        return Ok(CallToolResult::success(vec![Content::text(truncated)]));
    }

    let request = ExtractRequest::new(api_pages)
        .with_format("json".to_owned())
        .with_timeout_seconds(extract_timeout);

    if params.cache {
        if let Some(store) = cache_store {
            let key = generate_cache_key(&request);
            match store.get(&key).await {
                Ok(Some(cached_bytes)) => {
                    let mut cached_response: ExtractResponse =
                        serde_json::from_slice(&cached_bytes)
                            .map_err(|e| map_cache_error(e.into()))?;

                    if let Some(rules) = fallback_rules {
                        if let Some(data) = cached_response.data.as_mut() {
                            for item in data.iter_mut() {
                                if is_empty_content(item) {
                                    if let FallbackMatch::EmptyContent { message } =
                                        rules.check(&item.url)
                                    {
                                        item.markdown = Some(message);
                                    }
                                }
                            }
                        }
                    }

                    let mut merged_data: Vec<ExtractData> =
                        synthetic_data.into_iter().map(|(_, data)| data).collect();
                    if let Some(data) = cached_response.data {
                        merged_data.extend(data);
                    }
                    merged_data.sort_by(|a, b| {
                        let a_idx = original_pages
                            .iter()
                            .position(|p| p.url == a.url)
                            .unwrap_or(0);
                        let b_idx = original_pages
                            .iter()
                            .position(|p| p.url == b.url)
                            .unwrap_or(0);
                        a_idx.cmp(&b_idx)
                    });

                    let response = ExtractResponse {
                        meta: cached_response.meta,
                        data: if merged_data.is_empty() {
                            None
                        } else {
                            Some(merged_data)
                        },
                        errors: cached_response.errors,
                    };

                    let content = if params.output_format == "json" {
                        format_json(&response)
                    } else {
                        format_extract_markdown(&response)
                    };
                    let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
                    return Ok(CallToolResult::success(vec![Content::text(truncated)]));
                }
                Ok(None) => {}
                Err(e) => return Err(map_cache_error(e)),
            }
        }
    }

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }
        result = client.extract(request.clone()) => result,
    };

    match result {
        Ok(mut response) => {
            let _ =
                send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

            if let Some(rules) = fallback_rules {
                if let Some(data) = response.data.as_mut() {
                    for item in data.iter_mut() {
                        if is_empty_content(item) {
                            if let FallbackMatch::EmptyContent { message } = rules.check(&item.url)
                            {
                                item.markdown = Some(message);
                            }
                        }
                    }
                }
            }

            if let Some(store) = cache_store {
                let key = generate_cache_key(&request);
                let json_bytes =
                    serde_json::to_vec(&response).map_err(|e| map_cache_error(e.into()))?;
                store
                    .set(&key, "extract", &json_bytes)
                    .await
                    .map_err(map_cache_error)?;
            }

            let mut merged_data: Vec<ExtractData> =
                synthetic_data.into_iter().map(|(_, data)| data).collect();
            if let Some(data) = response.data {
                merged_data.extend(data);
            }
            merged_data.sort_by(|a, b| {
                let a_idx = original_pages
                    .iter()
                    .position(|p| p.url == a.url)
                    .unwrap_or(0);
                let b_idx = original_pages
                    .iter()
                    .position(|p| p.url == b.url)
                    .unwrap_or(0);
                a_idx.cmp(&b_idx)
            });

            let response = ExtractResponse {
                meta: response.meta,
                data: if merged_data.is_empty() {
                    None
                } else {
                    Some(merged_data)
                },
                errors: response.errors,
            };

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
