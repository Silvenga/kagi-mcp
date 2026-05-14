use super::{map_kagi_error, send_progress};
use crate::format::{format_extract_markdown, format_json};
use crate::guard::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use crate::validation::{validate_extract_pages_count, validate_extract_urls};
use kagi_api::types::{ExtractPage, ExtractRequest};
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
}

pub async fn extract_handler(
    client: &dyn KagiApi,
    params: ExtractParams,
    ctx: &RequestContext<RoleServer>,
    _extract_timeout: f64,
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

    let request = ExtractRequest {
        pages,
        format: Some("json".to_owned()),
    };

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
        result = client.extract(request) => result,
    };

    match result {
        Ok(response) => {
            let _ =
                send_progress(ctx, 100.0, Some(100.0), "Extraction completed.".to_owned()).await;
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
    use kagi_api::error::KagiError;
    use kagi_api::types::{ExtractData, ExtractError, ExtractResponse, Meta};
    use kagi_api::MockKagiApi;

    #[tokio::test]
    async fn when_zero_pages_should_return_invalid_params_error_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec![],
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

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
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

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
    async fn extract_success_returns_markdown() {
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
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("https://example.com"));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[tokio::test]
    async fn extract_success_json_returns_raw_json() {
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
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"data\""));
    }

    #[tokio::test]
    async fn extract_private_ip_rejected_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec!["https://192.168.1.1/".to_owned()],
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("URL validation failed"));
        assert!(err.to_string().contains("private IP"));
    }

    #[tokio::test]
    async fn extract_error_500_returns_server_error_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .returning(|_| Err(KagiError::ServerError));

        let params = ExtractParams {
            pages: vec!["https://example.com".to_owned()],
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Kagi API error"));
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    }

    #[tokio::test]
    async fn extract_partial_failure_renders_both_data_and_errors() {
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
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;

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
            .withf(|req| req.pages.len() == 1)
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
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx, 30.0).await;
        assert!(result.is_ok());
    }
}
