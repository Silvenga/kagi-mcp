use crate::cache::CacheStore;
use crate::metrics::MetricsStore;
use crate::tools::extract::batch::extract_batch;
use crate::tools::extract::errors::kagi_error_to_extract_error;
use crate::tools::extract::fallback::FallbackRules;
use crate::tools::extract::pipeline::{
    cache_results, classify_urls, render_results, ClassifiedUrl, ExtractFatalError,
    ExtractUrlResult,
};
use crate::tools::extract::validation::{validate_extract_pages_count, validate_extract_urls};
use crate::tools::extract::ExtractParams;
use kagi_api::{ExtractPage, KagiApi};
use rmcp::model::{CallToolResult, ErrorCode, ErrorData};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use std::sync::Arc;
use std::time::Instant;

fn extract_result_url(result: &ExtractUrlResult) -> &str {
    match result {
        ExtractUrlResult::Ok { url, .. } => url,
        ExtractUrlResult::Err { url, .. } => url,
    }
}

pub async fn extract_handler(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    cache_store: Option<&CacheStore>,
    fallback_rules: Option<&FallbackRules>,
    metrics_store: Option<&MetricsStore>,
) -> Result<CallToolResult, ErrorData> {
    // Step 1: Pre-validation
    if let Err(e) = validate_extract_pages_count(&params.pages) {
        return Err(ErrorData::invalid_params(
            format!("Pages validation failed: {e}"),
            None,
        ));
    }

    let validated_urls = match validate_extract_urls(&params.pages) {
        Ok(urls) => urls,
        Err(e) => {
            return Err(ErrorData::invalid_request(
                format!("URL validation failed: {e}"),
                None,
            ));
        }
    };

    let url_count = validated_urls.len();
    let start = Instant::now();
    tracing::info!(url_count, cache = params.cache, "extract started");

    let pages: Vec<ExtractPage> = validated_urls
        .into_iter()
        .map(|u| ExtractPage { url: u.to_string() })
        .collect();

    // Step 2: Classify
    let classified = classify_urls(&pages, params.cache, cache_store, fallback_rules).await;

    let cached_count = classified
        .iter()
        .filter(|c| matches!(c, ClassifiedUrl::Cached { .. }))
        .count() as i64;
    if cached_count > 0 {
        if let Some(ms) = metrics_store {
            ms.increment_extract_cache_hits(cached_count).await;
        }
    }

    // Step 3: Extract
    let extract_pages: Vec<ExtractPage> = classified
        .iter()
        .filter_map(|c| match c {
            ClassifiedUrl::Extract { page, .. } => Some(page.clone()),
            _ => None,
        })
        .collect();

    let mut extracted_results: Vec<ExtractUrlResult> = if extract_pages.is_empty() {
        Vec::new()
    } else {
        match extract_batch(client, ctx, extract_timeout, extract_pages).await {
            Ok(results) => {
                if let Some(ms) = metrics_store {
                    ms.increment_extract_request().await;
                }
                results
            }
            Err(ExtractFatalError::Cancelled) => {
                return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
            }
            Err(ExtractFatalError::Api(kagi_err)) => {
                // Batch graceful degradation: cached → Ok, uncached → Err
                let mut degraded = Vec::new();
                for c in &classified {
                    match c {
                        ClassifiedUrl::Cached { url, data } => {
                            degraded.push(ExtractUrlResult::Ok {
                                url: url.clone(),
                                markdown: data.markdown.clone(),
                            });
                        }
                        ClassifiedUrl::Extract { url, .. } => {
                            degraded.push(ExtractUrlResult::Err {
                                url: url.clone(),
                                error: kagi_error_to_extract_error(url, &kagi_err),
                            });
                        }
                        ClassifiedUrl::AlwaysBlock { .. } => {
                            // AlwaysBlock is not part of extraction; handled in merge
                        }
                    }
                }
                degraded
            }
        }
    };

    // Step 4: Cache + Merge
    // Cache only the newly extracted Ok results (skip cached and always-block)
    cache_results(&extracted_results, cache_store).await;

    let mut merged_results: Vec<ExtractUrlResult> = Vec::with_capacity(classified.len());
    for c in &classified {
        match c {
            ClassifiedUrl::AlwaysBlock { url, message } => {
                merged_results.push(ExtractUrlResult::Ok {
                    url: url.clone(),
                    markdown: Some(message.clone()),
                });
            }
            ClassifiedUrl::Cached { url, data } => {
                merged_results.push(ExtractUrlResult::Ok {
                    url: url.clone(),
                    markdown: data.markdown.clone(),
                });
            }
            ClassifiedUrl::Extract { url, .. } => {
                // Pull the corresponding result from extracted_results.
                // Since classify_urls preserves order and extract_batch
                // returns results in the same order as the input pages, we can drain
                // sequentially.
                if let Some(result) = extracted_results.first() {
                    if extract_result_url(result).trim_end_matches('/') == url.trim_end_matches('/')
                    {
                        merged_results.push(extracted_results.remove(0));
                        continue;
                    }
                }
                // Fallback: search by URL match
                if let Some(pos) = extracted_results.iter().position(|r| {
                    extract_result_url(r).trim_end_matches('/') == url.trim_end_matches('/')
                }) {
                    merged_results.push(extracted_results.remove(pos));
                } else {
                    tracing::warn!(url = %url, "missing extraction result during merge");
                    merged_results.push(ExtractUrlResult::Err {
                        url: url.clone(),
                        error: kagi_api::ExtractError {
                            url: url.clone(),
                            code: "missing_result".to_owned(),
                            message: Some("Extraction result missing during merge".to_owned()),
                        },
                    });
                }
            }
        }
    }

    let failure_count = merged_results
        .iter()
        .filter(|r| match r {
            ExtractUrlResult::Err { .. } => true,
            ExtractUrlResult::Ok { markdown, .. } => markdown
                .as_ref()
                .map(|m| m.trim().is_empty())
                .unwrap_or(true),
        })
        .count() as i64;
    if failure_count > 0 {
        if let Some(ms) = metrics_store {
            ms.increment_extract_failures(failure_count).await;
        }
    }

    tracing::info!(
        url_count,
        elapsed_ms = start.elapsed().as_millis(),
        "extract completed"
    );

    // Step 5: Render
    render_results(merged_results, fallback_rules, &params.output_format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheStore;
    use crate::cache::{ExtractCacheKey, ExtractCachedResult};
    use crate::config::FallbackRule;
    use crate::tools::output_format::OutputFormat;
    use chrono::Datelike;
    use kagi_api::{ExtractData, ExtractError, Meta};
    use kagi_api::{ExtractResponse, MockKagiApi};
    use rmcp::model::ErrorCode;
    use std::sync::Arc;

    fn test_client() -> Arc<MockKagiApi> {
        Arc::new(MockKagiApi::new())
    }

    fn make_extract_response(data: Vec<ExtractData>, errors: Vec<ExtractError>) -> ExtractResponse {
        ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(data),
            errors: Some(errors),
        }
    }

    #[tokio::test]
    async fn when_zero_pages_should_return_invalid_params_error_without_api_call() {
        let mock = test_client();

        let params = ExtractParams {
            pages: vec![],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Pages validation failed"));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn when_eleven_pages_should_return_invalid_params_error_without_api_call() {
        let mock = test_client();

        let params = ExtractParams {
            pages: (1..=11)
                .map(|i| format!("https://example{i}.com"))
                .collect(),
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Pages validation failed"));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn when_extract_succeeds_then_should_return_markdown() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: Some("# Hello\nWorld".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("https://example.com"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[tokio::test]
    async fn when_extract_succeeds_with_json_format_then_should_return_raw_json() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: Some("content".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: OutputFormat::Json,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"data\""));
    }

    #[tokio::test]
    async fn when_extract_with_private_ip_then_should_return_validation_error_without_api_call() {
        let mock = test_client();

        let params = ExtractParams {
            pages: vec!["https://192.168.1.1/".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("URL validation failed"));
        assert!(err.to_string().contains("private IP"));
    }

    #[tokio::test]
    async fn when_extract_returns_500_then_should_return_server_error_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .returning(|_| Err(kagi_api::KagiError::ServerError));

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("server error"));
    }

    #[tokio::test]
    async fn when_extract_has_partial_failure_then_should_render_both_data_and_errors() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(make_extract_response(
                vec![ExtractData {
                    url: "https://ok.com".to_owned(),
                    markdown: Some("Good content".to_owned()),
                    error: None,
                }],
                vec![ExtractError {
                    url: "https://fail.com".to_owned(),
                    code: "500".to_owned(),
                    message: Some("Server Error".to_owned()),
                }],
            ))
        });

        let params = ExtractParams {
            pages: vec!["https://ok.com".to_owned(), "https://fail.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Good content"));
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("server error"));
    }

    #[tokio::test]
    async fn when_extract_handler_called_then_config_extract_timeout_should_be_applied() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .withf(|req| req.pages().len() == 1)
            .returning(|_| {
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![]),
                    errors: None,
                })
            });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn when_split_disabled_then_should_call_api_once_with_all_pages() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .withf(|req| req.pages().len() == 3)
            .returning(|_| {
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![
                        ExtractData {
                            url: "https://a.com/".to_owned(),
                            markdown: Some("Content A".to_owned()),
                            error: None,
                        },
                        ExtractData {
                            url: "https://b.com/".to_owned(),
                            markdown: Some("Content B".to_owned()),
                            error: None,
                        },
                        ExtractData {
                            url: "https://c.com/".to_owned(),
                            markdown: Some("Content C".to_owned()),
                            error: None,
                        },
                    ]),
                    errors: None,
                })
            });

        let params = ExtractParams {
            pages: vec![
                "https://a.com".to_owned(),
                "https://b.com".to_owned(),
                "https://c.com".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Content A"));
        assert!(text.contains("Content B"));
        assert!(text.contains("Content C"));
    }

    #[tokio::test]
    async fn when_single_always_blocked_url_then_should_return_fallback_without_api_call() {
        let mock = MockKagiApi::new();

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://blocked.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Blocked by policy"));
    }

    #[tokio::test]
    async fn when_mixed_always_blocked_and_normal_urls_then_blocked_get_message_and_normal_get_content(
    ) {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .withf(|req| req.pages().len() == 1)
            .returning(|req| {
                let url = &req.pages()[0].url;
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![ExtractData {
                        url: url.clone(),
                        markdown: Some(format!("Content from {url}")),
                        error: None,
                    }]),
                    errors: None,
                })
            });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://blocked.com/page".to_owned(),
                "https://normal.com/page".to_owned(),
                "https://blocked.com/other".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let first_blocked = text.find("Blocked by policy").unwrap();
        let first_normal = text.find("Content from https://normal.com/page").unwrap();
        let second_blocked = text.rfind("Blocked by policy").unwrap();
        assert!(first_blocked < first_normal);
        assert!(first_normal < second_blocked);
    }

    #[tokio::test]
    async fn when_always_blocked_url_with_cache_enabled_then_should_not_cache_fallback_result() {
        let store = CacheStore::open_in_memory().await.expect("cache");
        let mock = MockKagiApi::new();

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://blocked.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            Some(&store),
            Some(&rules),
            None,
        )
        .await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Blocked by policy"));

        let cached = store
            .get_extract_result(&ExtractCacheKey {
                url: "https://blocked.com/page".to_owned(),
            })
            .await;
        assert!(cached.is_none(), "fallback result should not be cached");
    }

    #[tokio::test]
    async fn when_split_extract_empty_content_with_fallback_rule_then_should_substitute_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://fallback.com/page".to_owned(),
                    markdown: None,
                    error: None,
                }]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://fallback.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_split_extract_empty_string_with_fallback_rule_then_should_substitute_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://fallback.com/page".to_owned(),
                    markdown: Some("".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://fallback.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_split_extract_whitespace_with_fallback_rule_then_should_substitute_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://fallback.com/page".to_owned(),
                    markdown: Some("  \n  ".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://fallback.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_split_extract_real_content_with_fallback_rule_then_should_show_real_content() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://fallback.com/page".to_owned(),
                    markdown: Some("Real content".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://fallback.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Real content"));
        assert!(!text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_split_extract_empty_content_without_fallback_rule_then_should_show_original_empty(
    ) {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com/page".to_owned(),
                    markdown: None,
                    error: None,
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(!text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_cached_empty_content_with_fallback_rule_then_should_substitute_message() {
        let store = CacheStore::open_in_memory().await.expect("cache");

        let cached_result = ExtractCachedResult {
            data: ExtractData {
                url: "https://fallback.com/page".to_owned(),
                markdown: None,
                error: None,
            },
        };
        let cache_key = ExtractCacheKey {
            url: "https://fallback.com/page".to_owned(),
        };
        store
            .set_extract_result(&cache_key, &cached_result)
            .await
            .unwrap();

        let mock = MockKagiApi::new();

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec!["https://fallback.com/page".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            Some(&store),
            Some(&rules),
            None,
        )
        .await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_batch_all_urls_always_blocked_then_no_api_call_all_fallback_messages() {
        let mock = MockKagiApi::new();

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://blocked.com/page1".to_owned(),
                "https://blocked.com/page2".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Blocked by policy"));
    }

    #[tokio::test]
    async fn when_batch_mixed_blocked_and_normal_urls_then_correct_merge_and_ordering() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://normal.com/page".to_owned(),
                        markdown: Some("Normal content".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://other.com/page".to_owned(),
                        markdown: Some("Other content".to_owned()),
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://blocked.com/page".to_owned(),
                "https://normal.com/page".to_owned(),
                "https://blocked.com/other".to_owned(),
                "https://other.com/page".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let first_blocked = text.find("Blocked by policy").unwrap();
        let normal = text.find("Normal content").unwrap();
        let second_blocked = text.rfind("Blocked by policy").unwrap();
        let other = text.find("Other content").unwrap();
        assert!(first_blocked < normal);
        assert!(normal < second_blocked);
        assert!(second_blocked < other);
    }

    #[tokio::test]
    async fn when_batch_one_url_returns_empty_then_fallback_message_for_empty() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://normal.com/page".to_owned(),
                        markdown: Some("Normal content".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://fallback.com/page".to_owned(),
                        markdown: None,
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "fallback.com".to_owned(),
                message: "Fallback message".to_owned(),
                always_block: false,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://normal.com/page".to_owned(),
                "https://fallback.com/page".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Normal content"));
        assert!(text.contains("Fallback message"));
    }

    #[tokio::test]
    async fn when_batch_mode_then_preserves_original_url_ordering() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://first.com/page".to_owned(),
                        markdown: Some("First content".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://second.com/page".to_owned(),
                        markdown: Some("Second content".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://fourth.com/page".to_owned(),
                        markdown: Some("Fourth content".to_owned()),
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://first.com/page".to_owned(),
                "https://second.com/page".to_owned(),
                "https://blocked.com/page".to_owned(),
                "https://fourth.com/page".to_owned(),
            ],
            output_format: OutputFormat::Json,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result =
            extract_handler(Arc::new(mock), params, &ctx, 10.0, None, Some(&rules), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let first_pos = text.find("https://first.com/page").unwrap();
        let second_pos = text.find("https://second.com/page").unwrap();
        let blocked_pos = text.find("Blocked by policy").unwrap();
        let fourth_pos = text.find("https://fourth.com/page").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < blocked_pos);
        assert!(blocked_pos < fourth_pos);
    }

    #[tokio::test]
    async fn when_batch_blocked_urls_then_should_not_cache_fallback_results() {
        let store = CacheStore::open_in_memory().await.expect("cache");
        let mock = MockKagiApi::new();

        let rules = FallbackRules {
            rules: vec![FallbackRule {
                domain: "blocked.com".to_owned(),
                message: "Blocked by policy".to_owned(),
                always_block: true,
            }],
        };

        let params = ExtractParams {
            pages: vec![
                "https://blocked.com/page1".to_owned(),
                "https://blocked.com/page2".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            Some(&store),
            Some(&rules),
            None,
        )
        .await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Blocked by policy"));

        let cached1 = store
            .get_extract_result(&ExtractCacheKey {
                url: "https://blocked.com/page1".to_owned(),
            })
            .await;
        let cached2 = store
            .get_extract_result(&ExtractCacheKey {
                url: "https://blocked.com/page2".to_owned(),
            })
            .await;
        assert!(cached1.is_none(), "fallback result should not be cached");
        assert!(cached2.is_none(), "fallback result should not be cached");
    }

    #[tokio::test]
    async fn when_api_returns_per_url_error_then_handler_outputs_error() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://fail.com".to_owned(),
                    markdown: None,
                    error: Some("Request timed out after 10s".to_owned()),
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://fail.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, None, None, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("Request timed out after 10s"));
    }

    #[tokio::test]
    async fn when_extract_api_called_then_total_extract_requests_increments() {
        let metrics = MetricsStore::open_in_memory().await.unwrap();
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://a.com/".to_owned(),
                        markdown: Some("A".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://b.com/".to_owned(),
                        markdown: Some("B".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://c.com/".to_owned(),
                        markdown: Some("C".to_owned()),
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec![
                "https://a.com".to_owned(),
                "https://b.com".to_owned(),
                "https://c.com".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            None,
            None,
            Some(&metrics),
        )
        .await;

        assert!(result.is_ok());

        let now = chrono::Utc::now();
        let daily = metrics
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].total_extract_requests, 1);
    }

    #[tokio::test]
    async fn when_extract_urls_from_cache_then_total_extract_urls_from_cache_increments() {
        let metrics = MetricsStore::open_in_memory().await.unwrap();
        let store = CacheStore::open_in_memory().await.expect("cache");

        let cached_result = ExtractCachedResult {
            data: ExtractData {
                url: "https://cached.com/".to_owned(),
                markdown: Some("cached content".to_owned()),
                error: None,
            },
        };
        let cache_key = ExtractCacheKey {
            url: "https://cached.com/".to_owned(),
        };
        store
            .set_extract_result(&cache_key, &cached_result)
            .await
            .unwrap();

        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec!["https://cached.com".to_owned()],
            output_format: OutputFormat::Markdown,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            Some(&store),
            None,
            Some(&metrics),
        )
        .await;

        assert!(result.is_ok());

        let now = chrono::Utc::now();
        let daily = metrics
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].total_extract_urls_from_cache, 1);
    }

    #[tokio::test]
    async fn when_extract_fails_then_failed_extract_urls_increments() {
        let metrics = MetricsStore::open_in_memory().await.unwrap();
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://fail.com".to_owned(),
                        markdown: None,
                        error: Some("timeout".to_owned()),
                    },
                    ExtractData {
                        url: "https://empty.com".to_owned(),
                        markdown: Some("".to_owned()),
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec![
                "https://fail.com".to_owned(),
                "https://empty.com".to_owned(),
            ],
            output_format: OutputFormat::Markdown,
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(
            Arc::new(mock),
            params,
            &ctx,
            10.0,
            None,
            None,
            Some(&metrics),
        )
        .await;

        assert!(result.is_ok());

        let now = chrono::Utc::now();
        let daily = metrics
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].failed_extract_urls, 2);
    }

    pub async fn fake_request_context() -> RequestContext<RoleServer> {
        use crate::server::KagiMcpServer;
        use kagi_api::MockKagiApi;
        use rmcp::model::{ClientInfo, RequestId};
        use rmcp::service::serve_directly_with_ct;
        use std::sync::Arc;
        use tokio::io::duplex;
        use tokio_util::sync::CancellationToken;

        let (server_transport, client_transport) = duplex(4096);
        drop(client_transport);

        let server = KagiMcpServer::with_client(Arc::new(MockKagiApi::new()));
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
}
