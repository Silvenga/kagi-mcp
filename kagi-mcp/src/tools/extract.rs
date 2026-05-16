use super::{default_true, map_kagi_error, send_progress};
use crate::cache::error::CacheError;
use crate::cache::key::generate_cache_key;
use crate::cache::store::CacheStore;
use crate::format::{format_extract_markdown, format_json};
use crate::guard::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use crate::validation::{validate_extract_pages_count, validate_extract_urls};
use kagi_api::{ExtractPage, ExtractRequest, ExtractResponse};
use kagi_api::KagiApi;
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    pub pages: Vec<String>,
    pub output_format: Option<String>,
    /// Whether to use cached results. Default: true.
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub cache: bool,
}

fn map_cache_error(error: CacheError) -> ErrorData {
    ErrorData::internal_error(format!("Cache error: {error}"), None)
}

pub async fn extract_handler(
    client: &dyn KagiApi,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    _extract_timeout: f64,
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

    let request = ExtractRequest::new(pages).with_format("json".to_owned());

    if params.cache {
        if let Some(store) = cache_store {
            let key = generate_cache_key(&request);
            match store.get(&key) {
                Ok(Some(cached_bytes)) => {
                    let cached_response: ExtractResponse = serde_json::from_slice(&cached_bytes)
                        .map_err(|e| map_cache_error(e.into()))?;
                    let output_format = params.output_format.as_deref().unwrap_or("markdown");
                    let content = if output_format == "json" {
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
                    .map_err(map_cache_error)?;
            }

            let output_format = params.output_format.as_deref().unwrap_or("markdown");
            let content = if output_format == "json" {
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

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractData, ExtractError, ExtractResponse, KagiError, Meta};
    use kagi_api::MockKagiApi;

    #[tokio::test]
    async fn when_zero_pages_should_return_invalid_params_error_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec![],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Pages validation failed"));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn when_eleven_pages_should_return_invalid_params_error_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: (1..=11)
                .map(|i| format!("https://example{i}.com"))
                .collect(),
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Pages validation failed"));
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
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
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

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
            output_format: Some("json".to_owned()),
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"data\""));
    }

    #[tokio::test]
    async fn when_extract_with_private_ip_then_should_return_validation_error_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec!["https://192.168.1.1/".to_owned()],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

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
            .returning(|_| Err(KagiError::ServerError));

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

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
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;

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
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, None).await;
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
    async fn when_cache_hit_then_mock_api_should_not_be_called() {
        let mock = MockKagiApi::new();
        let store = CacheStore::open_in_memory().unwrap();

        let cached_response = make_extract_response(
            vec![ExtractData {
                url: "https://example.com/".to_owned(),
                markdown: Some("Cached content".to_owned()),
            }],
            vec![],
        );
        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com/".to_owned(),
        }])
        .with_format("json".to_owned());
        let key = generate_cache_key(&request);
        store
            .set(
                &key,
                "extract",
                &serde_json::to_vec(&cached_response).unwrap(),
            )
            .unwrap();

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, Some(&store)).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Cached content"));
    }

    #[tokio::test]
    async fn when_cache_miss_then_api_should_be_called_and_response_stored() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(make_extract_response(
                vec![ExtractData {
                    url: "https://example.com/".to_owned(),
                    markdown: Some("Fresh content".to_owned()),
                }],
                vec![],
            ))
        });

        let store = CacheStore::open_in_memory().unwrap();
        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, Some(&store)).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fresh content"));

        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com/".to_owned(),
        }])
        .with_format("json".to_owned());
        let key = generate_cache_key(&request);
        let cached = store.get(&key).unwrap();
        assert!(cached.is_some());
        let stored_response: ExtractResponse = serde_json::from_slice(&cached.unwrap()).unwrap();
        assert_eq!(
            stored_response.data.unwrap()[0].markdown,
            Some("Fresh content".to_owned())
        );
    }

    #[tokio::test]
    async fn when_cache_false_then_api_should_be_called_and_response_stored() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(make_extract_response(
                vec![ExtractData {
                    url: "https://example.com/".to_owned(),
                    markdown: Some("Fresh content".to_owned()),
                }],
                vec![],
            ))
        });

        let store = CacheStore::open_in_memory().unwrap();
        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
            cache: false,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, Some(&store)).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Fresh content"));

        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com/".to_owned(),
        }])
        .with_format("json".to_owned());
        let key = generate_cache_key(&request);
        let cached = store.get(&key).unwrap();
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn when_cache_corrupted_then_tool_call_should_fail() {
        let mock = MockKagiApi::new();
        let store = CacheStore::open_in_memory().unwrap();

        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com/".to_owned(),
        }])
        .with_format("json".to_owned());
        let key = generate_cache_key(&request);
        store.set(&key, "extract", b"invalid json").unwrap();

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
            cache: true,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0, Some(&store)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cache error"));
    }
}
