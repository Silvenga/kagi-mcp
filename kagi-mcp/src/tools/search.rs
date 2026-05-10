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

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub kagi_timeout: f64,
    pub limit: u32,
    pub safe_search: bool,
    pub region: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            kagi_timeout: 4.0,
            limit: 10,
            safe_search: true,
            region: None,
        }
    }
}

pub async fn search_handler(
    client: &dyn KagiApi,
    params: SearchParams,
    ctx: &RequestContext<RoleServer>,
    config: &SearchConfig,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let request = SearchRequest {
        query: params.query.clone(),
        workflow: params.workflow.clone(),
        format: Some("json".to_string()),
        timeout: Some(config.kagi_timeout),
        page: None,
        limit: Some(config.limit),
        safe_search: Some(config.safe_search),
        region: config.region.clone(),
        filters: build_filters(params.after, params.before, config.region.clone()),
    };

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Searching \"{}\"", params.query),
    )
    .await;

    if ctx.ct.is_cancelled() {
        return Err(rmcp::ErrorData::new(
            rmcp::model::ErrorCode(-32800),
            "Cancelled",
            None,
        ));
    }

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(rmcp::ErrorData::new(rmcp::model::ErrorCode(-32800), "Cancelled", None));
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

fn build_filters(
    after: Option<String>,
    before: Option<String>,
    region: Option<String>,
) -> Option<Filters> {
    if after.is_some() || before.is_some() || region.is_some() {
        Some(Filters {
            after,
            before,
            region,
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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert_eq!(content.len(), 1);
        let text = content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Example"));
        assert!(text.contains("https://example.com"));
        assert!(text.contains("Snippet text"));
    }

    #[tokio::test]
    async fn when_search_has_podcast_creator_then_result_should_include_podcast_creators_section() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(SearchResponse {
                meta: Meta {
                    trace: "test".to_string(),
                    node: None,
                    ms: None,
                },
                data: SearchData {
                    podcast_creator: Some(vec![SearchResult {
                        url: "https://example.com/pc".to_string(),
                        title: "Podcast Creator".to_string(),
                        snippet: Some("Top creator".to_string()),
                        time: Some("2024-06-01".to_string()),
                        image: None,
                        props: None,
                    }]),
                    ..empty_search_data()
                },
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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Podcast Creators"));
        assert!(text.contains("Podcast Creator"));
        assert!(text.contains("Top creator"));
        assert!(text.contains("2024-06-01"));
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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unauthorized"));
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

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

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid request"));
        assert!(err.to_string().contains("bad param"));
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn when_request_cancelled_should_return_error_code_32800() {
        let mut mock = MockKagiApi::new();
        // tokio::select! polls all branches; expectation prevents mock panic on poll
        mock.expect_search()
            .returning(|_| Err(KagiError::ServerError));

        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;
        ctx.ct.cancel();

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code.0, -32800);
        assert!(err.to_string().contains("Cancelled"));
    }

    #[tokio::test]
    async fn when_search_handler_called_then_server_config_should_be_applied_to_request() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .withf(|req| {
                req.limit == Some(25)
                    && req.safe_search == Some(false)
                    && req.region == Some("us-west".to_string())
                    && req.timeout == Some(8.5)
            })
            .returning(|_| Ok(make_search_response(vec![])));

        let config = SearchConfig {
            kagi_timeout: 8.5,
            limit: 25,
            safe_search: false,
            region: Some("us-west".to_string()),
        };
        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: None,
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn when_search_with_date_filters_then_filters_region_should_use_config_region() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .withf(|req| {
                req.filters
                    .as_ref()
                    .is_some_and(|f| f.region == Some("eu".to_string()))
            })
            .returning(|_| Ok(make_search_response(vec![])));

        let config = SearchConfig {
            kagi_timeout: 4.0,
            limit: 10,
            safe_search: true,
            region: Some("eu".to_string()),
        };
        let params = SearchParams {
            query: "test".to_string(),
            workflow: None,
            after: Some("2023-01-01".to_string()),
            before: None,
            output_format: None,
        };
        let ctx = super::super::test_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config).await;
        assert!(result.is_ok());
    }
}
