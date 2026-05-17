use crate::cache::{generate_cache_key, CacheError, CacheStore};
use crate::format::{format_extract_markdown, format_json};
use crate::guard::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use crate::tools::shared::{map_kagi_error, send_progress};
use crate::validation::{validate_extract_pages_count, validate_extract_urls};
use kagi_api::{ExtractPage, ExtractRequest, ExtractResponse, KagiApi};
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde::Deserialize;
use std::sync::Arc;
use tokio::task::JoinSet;

/// Parameters for the extract tool.
#[warn(missing_docs)]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    /// HTTPS URLs to extract content from. 1-10 URLs per call.
    pub pages: Vec<String>,
    /// Prefer 'markdown' for human-readable results optimized for LLM consumption.
    /// Use 'json' only when the caller explicitly requests raw structured data.
    #[serde(default = "crate::tools::shared::default_markdown")]
    #[schemars(default = "crate::tools::shared::default_markdown")]
    pub output_format: String,
    /// Whether to use cached results. Set to false only if freshness is critical.
    #[serde(default = "crate::tools::shared::default_true")]
    #[schemars(default = "crate::tools::shared::default_true")]
    pub cache: bool,
}

fn map_cache_error(error: CacheError) -> ErrorData {
    ErrorData::internal_error(format!("Cache error: {error}"), None)
}

fn kagi_error_to_extract_error(url: &str, error: kagi_api::KagiError) -> kagi_api::ExtractError {
    use kagi_api::KagiError;
    let (code, message) = match &error {
        KagiError::InvalidRequest { message: msg } => ("invalid_request", Some(msg.clone())),
        KagiError::Unauthorized => ("unauthorized", Some(error.to_string())),
        KagiError::Forbidden => ("forbidden", Some(error.to_string())),
        KagiError::RateLimited => ("rate_limited", Some(error.to_string())),
        KagiError::ServerError => ("server_error", Some(error.to_string())),
        KagiError::Network { source } => ("network_error", Some(source.to_string())),
        KagiError::Api {
            status,
            message: msg,
        } => ("api_error", Some(format!("HTTP {status}: {msg}"))),
    };
    kagi_api::ExtractError {
        url: url.to_owned(),
        code: code.to_owned(),
        message,
    }
}

pub async fn extract_handler(
    client: Arc<dyn KagiApi>,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    split_extract_requests: bool,
    cache_store: Option<&CacheStore>,
) -> Result<CallToolResult, ErrorData> {
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

    let pages: Vec<ExtractPage> = validated_urls
        .into_iter()
        .map(|u| ExtractPage { url: u.to_string() })
        .collect();

    if split_extract_requests {
        extract_split(client, params, ctx, extract_timeout, pages, cache_store).await
    } else {
        extract_batch(client, params, ctx, extract_timeout, pages, cache_store).await
    }
}

async fn extract_batch(
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

async fn extract_split(
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

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::MockKagiApi;
    use kagi_api::{ExtractData, ExtractError, Meta};
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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, true, None).await;

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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, true, None).await;

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
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

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
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: "json".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(mock, params, &ctx, 10.0, true, None).await;

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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, false, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Kagi API error"));
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn when_extract_has_partial_failure_then_should_render_both_data_and_errors() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(make_extract_response(
                vec![ExtractData {
                    url: "https://ok.com".to_owned(),
                    markdown: Some("Good content".to_owned()),
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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, false, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Good content"));
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("Server Error"));
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
            output_format: "markdown".to_owned(),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn when_extract_params_deserialized_without_cache_should_default_to_true() {
        let json = r#"{"pages": ["https://example.com"]}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_extract_params_deserialized_with_cache_false_should_be_false() {
        let json = r#"{"pages": ["https://example.com"], "cache": false}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(!params.cache);
    }

    #[test]
    fn when_extract_params_deserialized_with_cache_true_should_be_true() {
        let json = r#"{"pages": ["https://example.com"], "cache": true}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[tokio::test]
    async fn when_split_extract_two_urls_then_api_called_twice_with_single_page_requests() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(2)
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
                    }]),
                    errors: None,
                })
            });

        let params = ExtractParams {
            pages: vec!["https://a.com".to_owned(), "https://b.com".to_owned()],
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("https://a.com"));
        assert!(text.contains("Content from https://a.com"));
        assert!(text.contains("https://b.com"));
        assert!(text.contains("Content from https://b.com"));
    }

    #[tokio::test]
    async fn when_split_extract_one_fails_then_error_appears_in_output() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(2).returning(|req| {
            let url = &req.pages()[0].url;
            if url == "https://fail.com/" {
                Err(kagi_api::KagiError::ServerError)
            } else {
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![ExtractData {
                        url: url.clone(),
                        markdown: Some(format!("Content from {url}")),
                    }]),
                    errors: None,
                })
            }
        });

        let params = ExtractParams {
            pages: vec!["https://ok.com".to_owned(), "https://fail.com".to_owned()],
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Content from https://ok.com/"));
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("server error"));
    }

    #[tokio::test]
    async fn when_split_extract_results_maintain_input_order() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(3).returning(|req| {
            let url = &req.pages()[0].url;
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: url.clone(),
                    markdown: Some(url.clone()),
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec![
                "https://first.com".to_owned(),
                "https://second.com".to_owned(),
                "https://third.com".to_owned(),
            ],
            output_format: "json".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let first_pos = text.find("https://first.com/").unwrap();
        let second_pos = text.find("https://second.com/").unwrap();
        let third_pos = text.find("https://third.com/").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    #[tokio::test]
    async fn when_split_enabled_then_should_call_api_per_url() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(3)
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
                    }]),
                    errors: None,
                })
            });

        let params = ExtractParams {
            pages: vec![
                "https://a.com".to_owned(),
                "https://b.com".to_owned(),
                "https://c.com".to_owned(),
            ],
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Content from https://a.com/"));
        assert!(text.contains("Content from https://b.com/"));
        assert!(text.contains("Content from https://c.com/"));
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
                        },
                        ExtractData {
                            url: "https://b.com/".to_owned(),
                            markdown: Some("Content B".to_owned()),
                        },
                        ExtractData {
                            url: "https://c.com/".to_owned(),
                            markdown: Some("Content C".to_owned()),
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
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, false, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Content A"));
        assert!(text.contains("Content B"));
        assert!(text.contains("Content C"));
    }

    #[tokio::test]
    async fn when_split_enabled_with_partial_failure_then_should_aggregate_successes_and_errors() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(3).returning(|req| {
            let url = &req.pages()[0].url;
            if url == "https://fail.com/" {
                Err(kagi_api::KagiError::ServerError)
            } else {
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![ExtractData {
                        url: url.clone(),
                        markdown: Some(format!("Content from {url}")),
                    }]),
                    errors: None,
                })
            }
        });

        let params = ExtractParams {
            pages: vec![
                "https://ok1.com".to_owned(),
                "https://fail.com".to_owned(),
                "https://ok2.com".to_owned(),
            ],
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Content from https://ok1.com/"));
        assert!(text.contains("Content from https://ok2.com/"));
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("server error"));
    }

    #[tokio::test]
    async fn when_single_url_with_split_enabled_then_should_call_api_once() {
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
                    data: Some(vec![ExtractData {
                        url: "https://only.com/".to_owned(),
                        markdown: Some("Single content".to_owned()),
                    }]),
                    errors: None,
                })
            });

        let params = ExtractParams {
            pages: vec!["https://only.com".to_owned()],
            output_format: "markdown".to_owned(),
            cache: false,
        };
        let ctx = fake_request_context().await;

        let result = extract_handler(Arc::new(mock), params, &ctx, 10.0, true, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Single content"));
    }

    #[test]
    fn when_extract_params_deserialized_without_output_format_then_should_default_to_markdown() {
        let json = r#"{"pages": ["https://example.com"]}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.output_format, "markdown");
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
