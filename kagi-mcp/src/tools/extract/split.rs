use crate::cache::{CacheStore, ExtractCacheKey, ExtractCachedResult};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::extract::errors::kagi_error_to_extract_error;
use crate::tools::extract::fallback::{is_empty_content, FallbackMatch, FallbackRules};
use crate::tools::extract::ExtractParams;
use crate::tools::progress::send_progress;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractData, ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

fn apply_post_extract_fallback(
    response: &mut ExtractResponse,
    fallback_rules: Option<&FallbackRules>,
) {
    if let Some(rules) = fallback_rules {
        if let Some(data) = response.data.as_mut() {
            for item in data.iter_mut() {
                if is_empty_content(item) {
                    if let FallbackMatch::EmptyContent { message } = rules.check(&item.url) {
                        item.markdown = Some(message);
                    }
                }
            }
        }
    }
}

pub async fn extract_split(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
    cache_store: Option<&CacheStore>,
    fallback_rules: Option<&FallbackRules>,
) -> Result<CallToolResult, ErrorData> {
    let total_pages = pages.len();

    let start = Instant::now();
    tracing::info!(total_pages = total_pages, "extract split started");

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
        if let Some(rules) = fallback_rules {
            if let FallbackMatch::AlwaysBlock { message } = rules.check(&page.url) {
                let _ = send_progress(
                    ctx,
                    ((i + 1) as f64 / total_pages as f64) * 100.0,
                    Some(100.0),
                    format!("Page {}/{} (blocked)", i + 1, total_pages),
                )
                .await;
                results[i] = Some(ExtractResponse {
                    meta: kagi_api::Meta {
                        trace: String::new(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![ExtractData {
                        url: page.url.clone(),
                        markdown: Some(message),
                    }]),
                    errors: None,
                });
                continue;
            }
        }

        let mut cache_hit = false;
        if params.cache {
            if let Some(store) = cache_store {
                let cache_key = ExtractCacheKey {
                    url: page.url.clone(),
                };
                if let Some(cached_result) = store.get_extract_result(&cache_key).await {
                    let _ = send_progress(
                        ctx,
                        ((i + 1) as f64 / total_pages as f64) * 100.0,
                        Some(100.0),
                        format!("Page {}/{} (cached)", i + 1, total_pages),
                    )
                    .await;
                    let mut cached_response = ExtractResponse {
                        meta: kagi_api::Meta {
                            trace: String::new(),
                            node: None,
                            ms: None,
                        },
                        data: Some(vec![cached_result.data]),
                        errors: None,
                    };
                    apply_post_extract_fallback(&mut cached_response, fallback_rules);
                    results[i] = Some(cached_response);
                    cache_hit = true;
                    tracing::info!(url = %page.url, cache_hit = true, "page served from cache");
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
                        if let Some(data_vec) = &api_response.data {
                            if let Some(extracted_data) =
                                data_vec.iter().find(|d| {
                                    d.url.trim_end_matches('/')
                                        == pages[idx].url.trim_end_matches('/')
                                })
                            {
                                let cache_key = ExtractCacheKey {
                                    url: pages[idx].url.clone(),
                                };
                                let cached_result = ExtractCachedResult {
                                    data: extracted_data.clone(),
                                };
                                let _ = store.set_extract_result(&cache_key, &cached_result).await;
                            }
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
                let mut api_response = api_response;
                apply_post_extract_fallback(&mut api_response, fallback_rules);
                results[idx] = Some(api_response);
                tracing::info!(url = %pages[idx].url, cache_hit = false, "page extracted");
            }
            Ok((idx, Err(kagi_err))) => {
                match &kagi_err {
                    kagi_api::KagiError::Unauthorized
                    | kagi_api::KagiError::InvalidRequest { .. } => {
                        tracing::error!(url = %pages[idx].url, error = %kagi_err, "page extraction failed");
                    }
                    _ => {
                        tracing::warn!(url = %pages[idx].url, error = %kagi_err, "page extraction failed");
                    }
                }
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

    let data_len = data.len();
    let errors_len = errors.len();

    tracing::info!(
        success_count = data_len,
        error_count = errors_len,
        elapsed_ms = start.elapsed().as_millis(),
        "extract split completed"
    );

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
