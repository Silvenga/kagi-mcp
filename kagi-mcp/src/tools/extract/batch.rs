use crate::cache::{CacheStore, ExtractCacheKey, ExtractCachedResult};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::errors::map_kagi_error;
use crate::tools::extract::fallback::{is_empty_content, FallbackMatch, FallbackRules};
use crate::tools::extract::ExtractParams;
use crate::tools::progress::send_progress;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractData, ExtractError, ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer};
use std::sync::Arc;
use std::time::Instant;

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

    let start = Instant::now();
    tracing::info!(total_pages = total_pages, "extract batch started");

    // Phase 0 + Phase 1: fallback filtering + per-URL cache lookup
    let mut results: Vec<Option<ExtractData>> = vec![None; total_pages];
    let mut uncached: Vec<(usize, ExtractPage)> = Vec::new();

    let mut cache_hits_count = 0usize;

    for (i, page) in pages.iter().enumerate() {
        // Phase 0: always_block URLs get synthetic data, skip cache + API
        if let Some(rules) = fallback_rules {
            if let FallbackMatch::AlwaysBlock { message } = rules.check(&page.url) {
                results[i] = Some(ExtractData {
                    url: page.url.clone(),
                    markdown: Some(message),
                });
                continue;
            }
        }

        // Phase 1: per-URL cache lookup
        if params.cache {
            if let Some(store) = cache_store {
                let key = ExtractCacheKey {
                    url: page.url.clone(),
                };
                if let Some(cached) = store.get_extract_result(&key).await {
                    results[i] = Some(cached.data);
                    cache_hits_count += 1;
                    tracing::info!(url = %page.url, cache_hit = true, "page served from cache");
                    continue;
                }
                tracing::debug!(url = %page.url, "cache miss");
            }
        }

        uncached.push((i, page.clone()));
    }

    // All URLs were either blocked or cached — return immediately
    if uncached.is_empty() {
        let _ = send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

        let mut merged_data: Vec<ExtractData> =
            results.iter_mut().filter_map(|r| r.take()).collect();

        apply_post_extract_fallback(&mut merged_data, fallback_rules);

        let response = ExtractResponse {
            meta: kagi_api::Meta {
                trace: String::new(),
                node: None,
                ms: None,
            },
            data: if merged_data.is_empty() {
                None
            } else {
                Some(merged_data)
            },
            errors: None,
        };

        let content = if params.output_format == "json" {
            format_json(&response)
        } else {
            format_extract_markdown(&response)
        };
        let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        let cache_hit_label = if cache_hits_count > 0 { "true" } else { "fallback" };
        tracing::info!(
            cache_hit = cache_hit_label,
            url_count = total_pages,
            "extract batch served from cache"
        );
        return Ok(CallToolResult::success(vec![Content::text(truncated)]));
    }

    // Phase 2: single batch API call for uncached URLs only
    let api_pages: Vec<ExtractPage> = uncached.iter().map(|(_, page)| page.clone()).collect();
    let request = ExtractRequest::new(api_pages)
        .with_format("json".to_owned())
        .with_timeout_seconds(extract_timeout);

    tracing::debug!(
        url_count = uncached.len(),
        "batch cache miss, calling Kagi API"
    );

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }
        result = client.extract(request) => result,
    };

    match result {
        Ok(mut response) => {
            let _ =
                send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;

            // Phase 3: cache each successful ExtractData individually
            if let Some(store) = cache_store {
                if let Some(data) = &response.data {
                    for item in data {
                        let key = ExtractCacheKey {
                            url: item.url.clone(),
                        };
                        let cached = ExtractCachedResult { data: item.clone() };
                        let _ = store.set_extract_result(&key, &cached).await;
                    }
                }
            }

            if let Some(data) = response.data.take() {
                for item in data {
                    if let Some((idx, _)) = uncached.iter().find(|(_, p)| {
                        p.url.trim_end_matches('/') == item.url.trim_end_matches('/')
                    }) {
                        results[*idx] = Some(item);
                    }
                }
            }

            // Build final merged data preserving input order
            let mut merged_data: Vec<ExtractData> =
                results.iter_mut().filter_map(|r| r.take()).collect();

            apply_post_extract_fallback(&mut merged_data, fallback_rules);

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
            tracing::info!(
                cache_hit = false,
                elapsed_ms = start.elapsed().as_millis(),
                "extract batch completed"
            );
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => {
            match &e {
                kagi_api::KagiError::Unauthorized | kagi_api::KagiError::InvalidRequest { .. } => {
                    tracing::error!(error = %e, "extract batch failed");
                }
                _ => {
                    tracing::warn!(error = %e, "extract batch failed");
                }
            }

            // If we have cached results, return them with error entries for uncached URLs
            let has_cached = results.iter().any(|r| r.is_some());
            if has_cached {
                let _ = send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned())
                    .await;

                let mut errors: Vec<ExtractError> = Vec::new();
                for (_, page) in &uncached {
                    errors.push(ExtractError {
                        url: page.url.clone(),
                        code: "extract_failed".to_owned(),
                        message: Some(e.to_string()),
                    });
                }

                let mut merged_data: Vec<ExtractData> =
                    results.iter_mut().filter_map(|r| r.take()).collect();

                apply_post_extract_fallback(&mut merged_data, fallback_rules);

                let response = ExtractResponse {
                    meta: kagi_api::Meta {
                        trace: String::new(),
                        node: None,
                        ms: None,
                    },
                    data: if merged_data.is_empty() {
                        None
                    } else {
                        Some(merged_data)
                    },
                    errors: if errors.is_empty() {
                        None
                    } else {
                        Some(errors)
                    },
                };

                let content = if params.output_format == "json" {
                    format_json(&response)
                } else {
                    format_extract_markdown(&response)
                };
                let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
                return Ok(CallToolResult::success(vec![Content::text(truncated)]));
            }

            Err(map_kagi_error(e))
        }
    }
}

fn apply_post_extract_fallback(data: &mut [ExtractData], fallback_rules: Option<&FallbackRules>) {
    if let Some(rules) = fallback_rules {
        for item in data.iter_mut() {
            if is_empty_content(item) {
                if let FallbackMatch::EmptyContent { message } = rules.check(&item.url) {
                    item.markdown = Some(message);
                }
            }
        }
    }
}
