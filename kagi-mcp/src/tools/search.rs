use rmcp::model::{CallToolResult, Content};
use rmcp::schemars;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde::Deserialize;

use kagi_api::types::{Filters, SearchRequest};
use kagi_api::KagiApi;

use super::{map_kagi_error, send_progress};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    pub query: String,
    pub workflow: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub output_format: Option<String>,
}

pub async fn search_handler(
    client: &dyn KagiApi,
    params: SearchParams,
    ctx: &RequestContext<RoleServer>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let request = SearchRequest {
        query: params.query.clone(),
        workflow: params.workflow.clone(),
        format: Some("json".to_string()),
        timeout: None,
        page: None,
        limit: None,
        safe_search: None,
        region: None,
        filters: build_filters(params.after, params.before),
    };

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Searching \"{}\"", params.query),
    )
    .await;

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(rmcp::ErrorData::internal_error("Cancelled", None));
        }
        result = client.search(request) => result,
    };

    let _ = send_progress(ctx, 100.0, Some(100.0), "Query completed.".to_string()).await;

    match result {
        Ok(response) => {
            let output_format = params.output_format.as_deref().unwrap_or("markdown");
            let content = if output_format == "json" {
                crate::format::format_json(&response)
            } else {
                crate::format::format_search_markdown(&response)
            };
            let truncated =
                crate::guard::truncate_response(&content, crate::guard::DEFAULT_MAX_RESPONSE_BYTES);
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Err(map_kagi_error(e)),
    }
}

fn build_filters(after: Option<String>, before: Option<String>) -> Option<Filters> {
    if after.is_some() || before.is_some() {
        Some(Filters {
            after,
            before,
            region: None,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::error::KagiError;
    use kagi_api::types::{Meta, SearchData, SearchResponse, SearchResult};
    use kagi_api::MockKagiApi;

    fn empty_search_data() -> SearchData {
        SearchData {
            search: None,
            image: None,
            video: None,
            podcast: None,
            podcast_creator: None,
            news: None,
            adjacent_question: None,
            direct_answer: None,
            interesting_news: None,
            interesting_finds: None,
            infobox: None,
            code: None,
            package_tracking: None,
            public_records: None,
            weather: None,
            related_search: None,
            listicle: None,
            web_archive: None,
        }
    }

    fn make_search_response(results: Vec<SearchResult>) -> SearchResponse {
        SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(results),
                ..empty_search_data()
            },
        }
    }

    #[tokio::test]
    async fn search_success_returns_markdown() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(make_search_response(vec![SearchResult {
                url: "https://example.com".to_string(),
                title: "Example".to_string(),
                snippet: Some("Snippet text".to_string()),
                time: Some("2023-01-01".to_string()),
                image: None,
                props: None,
            }]))
        });

        let params = SearchParams {
            query: "test query".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert_eq!(content.len(), 1);
        let text = content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Example"));
        assert!(text.contains("https://example.com"));
        assert!(text.contains("Snippet text"));
    }

    #[tokio::test]
    async fn search_success_json_returns_raw_json() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(make_search_response(vec![SearchResult {
                url: "https://example.com".to_string(),
                title: "Example".to_string(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]))
        });

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: Some("json".to_string()),
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"search\""));
    }

    #[tokio::test]
    async fn search_empty_results_returns_no_results_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(SearchResponse {
                meta: Meta {
                    trace: "test".to_string(),
                    node: None,
                    ms: None,
                },
                data: empty_search_data(),
            })
        });

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert_eq!(text, "No results found.");
    }

    #[tokio::test]
    async fn search_error_401_returns_unauthorized_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .returning(|_| Err(KagiError::Unauthorized));

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn search_error_429_returns_rate_limited_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .returning(|_| Err(KagiError::RateLimited));

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Rate limited"));
    }

    #[tokio::test]
    async fn search_invalid_request_returns_error_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Err(KagiError::InvalidRequest {
                message: "bad param".to_string(),
            })
        });

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid request"));
        assert!(err.to_string().contains("bad param"));
    }
}
