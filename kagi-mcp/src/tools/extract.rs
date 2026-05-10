use rmcp::model::{CallToolResult, Content};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde::Deserialize;

use kagi_api::types::{ExtractPage, ExtractRequest};
use kagi_api::KagiApi;

use super::{map_kagi_error, send_progress};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    pub pages: Vec<String>,
    pub timeout: Option<f64>,
    pub output_format: Option<String>,
}

pub async fn extract_handler(
    client: &dyn KagiApi,
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
        .map(|u| ExtractPage { url: u.to_string() })
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

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::error::KagiError;
    use kagi_api::types::{ExtractData, ExtractError, ExtractResponse, Meta};
    use kagi_api::MockKagiApi;

    fn make_extract_response(data: Vec<ExtractData>, errors: Vec<ExtractError>) -> ExtractResponse {
        ExtractResponse {
            meta: Meta {
                trace: "test".to_string(),
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
                    trace: "test".to_string(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_string(),
                    markdown: Some("# Hello\nWorld".to_string()),
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_string()],
            timeout: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx).await;

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
                    trace: "test".to_string(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_string(),
                    markdown: Some("content".to_string()),
                }]),
                errors: None,
            })
        });

        let params = ExtractParams {
            pages: vec!["https://example.com".to_string()],
            timeout: None,
            output_format: Some("json".to_string()),
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"data\""));
    }

    #[tokio::test]
    async fn extract_private_ip_rejected_without_api_call() {
        let mock = MockKagiApi::new();

        let params = ExtractParams {
            pages: vec!["https://192.168.1.1/".to_string()],
            timeout: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx).await;

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
            pages: vec!["https://example.com".to_string()],
            timeout: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Kagi API error"));
    }

    #[tokio::test]
    async fn extract_partial_failure_renders_both_data_and_errors() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(make_extract_response(
                vec![ExtractData {
                    url: "https://ok.com".to_string(),
                    markdown: Some("Good content".to_string()),
                }],
                vec![ExtractError {
                    url: "https://fail.com".to_string(),
                    code: "500".to_string(),
                    message: Some("Server Error".to_string()),
                }],
            ))
        });

        let params = ExtractParams {
            pages: vec!["https://ok.com".to_string(), "https://fail.com".to_string()],
            timeout: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = extract_handler(&mock, params, &ctx).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Good content"));
        assert!(text.contains("https://fail.com"));
        assert!(text.contains("Server Error"));
    }
}
