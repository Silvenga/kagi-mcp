use crate::cache::{generate_cache_key, CacheError, CacheStore};
use crate::format::{format_json, format_search_markdown};
use crate::tools::errors::map_kagi_error;
use crate::tools::progress::send_progress;
use crate::tools::search::dedup::dedup_by_domain;
use crate::tools::search::SearchParams;
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{Filters, KagiApi, SearchRequest, SearchResponse};
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub search_timeout: f64,
    pub limit: u32,
    pub safe_search: bool,
    pub region: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            search_timeout: 4.0,
            limit: 1024,
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
    cache_store: Option<&CacheStore>,
) -> Result<CallToolResult, ErrorData> {
    if params.limit_per_domain == Some(0) {
        return Err(ErrorData::invalid_request(
            "limit_per_domain must be >= 1",
            None,
        ));
    }

    let upstream_limit = config.limit;

    let mut request = SearchRequest::new(params.query.clone())
        .with_format("json".to_owned())
        .with_timeout_seconds(config.search_timeout)
        .with_limit(upstream_limit)
        .with_safe_search(config.safe_search);

    if let Some(workflow) = params.workflow.clone() {
        request = request.with_workflow(workflow);
    }
    if let Some(region) = config.region.clone() {
        request = request.with_region(region);
    }
    if let Some(filters) = build_filters(params.after, params.before, config.region.clone()) {
        request = request.with_filters(filters);
    }

    if params.cache {
        if let Some(store) = cache_store {
            let key = generate_cache_key(&request);
            match store.get(&key).await {
                Ok(Some(cached_bytes)) => {
                    let mut cached_response: SearchResponse = serde_json::from_slice(&cached_bytes)
                        .map_err(|e| map_cache_error(e.into()))?;
                    if let Some(lpd) = params.limit_per_domain {
                        dedup_by_domain(&mut cached_response.data, lpd, config.limit);
                    }
                    let content = if params.output_format == "json" {
                        format_json(&cached_response)
                    } else {
                        format_search_markdown(&cached_response)
                    };
                    let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
                    return Ok(CallToolResult::success(vec![Content::text(truncated)]));
                }
                Ok(None) => {}
                Err(e) => return Err(map_cache_error(e)),
            }
        }
    }

    let _ = send_progress(
        ctx,
        0.0,
        Some(100.0),
        format!("Searching \"{}\"", params.query),
    )
    .await;

    if ctx.ct.is_cancelled() {
        return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
    }

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(ErrorData::new(ErrorCode(-32800), "Cancelled", None));
        }
        result = client.search(request.clone()) => result,
    };

    let _ = send_progress(ctx, 100.0, Some(100.0), "Query completed.".to_owned()).await;

    match result {
        Ok(mut response) => {
            if let Some(store) = cache_store {
                let key = generate_cache_key(&request);
                let json_bytes =
                    serde_json::to_vec(&response).map_err(|e| map_cache_error(e.into()))?;
                store
                    .set(&key, "search", &json_bytes)
                    .await
                    .map_err(map_cache_error)?;
            }

            if let Some(lpd) = params.limit_per_domain {
                dedup_by_domain(&mut response.data, lpd, config.limit);
            }
            let content = if params.output_format == "json" {
                format_json(&response)
            } else {
                format_search_markdown(&response)
            };
            let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Err(map_kagi_error(e)),
    }
}

fn map_cache_error(error: CacheError) -> ErrorData {
    ErrorData::internal_error(format!("Cache error: {error}"), None)
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
    use crate::server::KagiMcpServer;
    use kagi_api::MockKagiApi;
    use kagi_api::{KagiError, Meta, SearchData, SearchResponse, SearchResult};
    use rmcp::model::{ClientInfo, RequestId};
    use rmcp::service::serve_directly_with_ct;
    use std::sync::Arc;
    use tokio::io::duplex;
    use tokio_util::sync::CancellationToken;

    fn fake_search_response(results: Vec<SearchResult>) -> SearchResponse {
        SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
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
    async fn when_search_succeeds_then_should_return_markdown() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(fake_search_response(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("Snippet text".to_owned()),
                time: Some("2023-01-01".to_owned()),
                image: None,
                props: None,
            }]))
        });

        let params = SearchParams {
            query: "test query".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

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
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: SearchData {
                    podcast_creator: Some(vec![SearchResult {
                        url: "https://example.com/pc".to_owned(),
                        title: "Podcast Creator".to_owned(),
                        snippet: Some("Top creator".to_owned()),
                        time: Some("2024-06-01".to_owned()),
                        image: None,
                        props: None,
                    }]),
                    ..empty_search_data()
                },
            })
        });

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Podcast Creators"));
        assert!(text.contains("Podcast Creator"));
        assert!(text.contains("Top creator"));
        assert!(text.contains("2024-06-01"));
    }

    #[tokio::test]
    async fn when_search_succeeds_with_json_format_then_should_return_raw_json() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(fake_search_response(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]))
        });

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("\"trace\""));
        assert!(text.contains("\"search\""));
    }

    #[tokio::test]
    async fn when_search_has_no_results_then_should_return_no_results_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(SearchResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: empty_search_data(),
            })
        });

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert_eq!(text, "No results found.");
    }

    #[tokio::test]
    async fn when_search_returns_401_then_should_return_unauthorized_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .returning(|_| Err(KagiError::Unauthorized));

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unauthorized"));
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn when_search_returns_429_then_should_return_rate_limited_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search()
            .times(1)
            .returning(|_| Err(KagiError::RateLimited));

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Rate limited"));
    }

    #[tokio::test]
    async fn when_search_returns_invalid_request_then_should_return_error_message() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Err(KagiError::InvalidRequest {
                message: "bad param".to_owned(),
            })
        });

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid request"));
        assert!(err.to_string().contains("bad param"));
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
    }

    #[tokio::test]
    async fn when_request_cancelled_should_return_error_code_32800() {
        let mut mock = MockKagiApi::new();
        // tokio::select! polls all branches; expectation prevents mock panic on poll
        mock.expect_search()
            .returning(|_| Err(KagiError::ServerError));

        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: None,
            cache: true,
        };
        let ctx = fake_request_context().await;
        ctx.ct.cancel();

        let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), None).await;

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
                req.limit() == Some(25)
                    && req.safe_search() == Some(false)
                    && req.region() == Some("us-west")
                    && req.timeout_seconds() == Some(8.5)
            })
            .returning(|_| Ok(fake_search_response(vec![])));

        let config = SearchConfig {
            search_timeout: 8.5,
            limit: 25,
            safe_search: false,
            region: Some("us-west".to_owned()),
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn when_limit_per_domain_applied_then_categories_should_have_independent_counters() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(SearchResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: SearchData {
                    search: Some(vec![SearchResult {
                        url: "https://example.com/s1".to_owned(),
                        title: "Search 1".to_owned(),
                        snippet: None,
                        time: None,
                        image: None,
                        props: None,
                    }]),
                    news: Some(vec![SearchResult {
                        url: "https://example.com/n1".to_owned(),
                        title: "News 1".to_owned(),
                        snippet: None,
                        time: None,
                        image: None,
                        props: None,
                    }]),
                    ..empty_search_data()
                },
            })
        });

        let config = SearchConfig {
            limit: 10,
            ..SearchConfig::default()
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["data"]["search"].as_array().unwrap().len() == 1);
        assert!(parsed["data"]["news"].as_array().unwrap().len() == 1);
    }

    #[tokio::test]
    async fn when_dedup_applied_then_original_order_should_be_preserved() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(fake_search_response(vec![
                SearchResult {
                    url: "https://a.com/1".to_owned(),
                    title: "A1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://b.com/1".to_owned(),
                    title: "B1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://a.com/2".to_owned(),
                    title: "A2".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
            ]))
        });

        let config = SearchConfig {
            limit: 10,
            ..SearchConfig::default()
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let search = parsed["data"]["search"].as_array().unwrap();
        assert_eq!(search.len(), 2);
        assert_eq!(search[0]["title"], "A1");
        assert_eq!(search[1]["title"], "B1");
    }

    #[tokio::test]
    async fn when_props_group_id_present_then_should_dedup_by_it_over_etld1() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(fake_search_response(vec![
                SearchResult {
                    url: "https://blog.example.com/1".to_owned(),
                    title: "Blog 1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: Some(serde_json::json!({"group_id": "blog.example.com"})),
                },
                SearchResult {
                    url: "https://www.example.com/1".to_owned(),
                    title: "Main 1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: Some(serde_json::json!({"group_id": "www.example.com"})),
                },
                SearchResult {
                    url: "https://blog.example.com/2".to_owned(),
                    title: "Blog 2".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: Some(serde_json::json!({"group_id": "blog.example.com"})),
                },
            ]))
        });

        let config = SearchConfig {
            limit: 10,
            ..SearchConfig::default()
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let search = parsed["data"]["search"].as_array().unwrap();
        assert_eq!(search.len(), 2);
        assert_eq!(search[0]["title"], "Blog 1");
        assert_eq!(search[1]["title"], "Main 1");
    }

    #[tokio::test]
    async fn when_final_count_after_dedup_exceeds_limit_then_should_truncate_to_limit() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(fake_search_response(vec![
                SearchResult {
                    url: "https://a.com/1".to_owned(),
                    title: "A1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://b.com/1".to_owned(),
                    title: "B1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://c.com/1".to_owned(),
                    title: "C1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
            ]))
        });

        let config = SearchConfig {
            search_timeout: 4.0,
            limit: 2,
            safe_search: true,
            region: None,
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "json".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let search = parsed["data"]["search"].as_array().unwrap();
        assert_eq!(search.len(), 2);
        assert_eq!(search[0]["title"], "A1");
        assert_eq!(search[1]["title"], "B1");
    }

    #[tokio::test]
    async fn when_limit_per_domain_set_but_no_results_then_should_handle_gracefully() {
        let mut mock = MockKagiApi::new();
        mock.expect_search().times(1).returning(|_| {
            Ok(SearchResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: empty_search_data(),
            })
        });

        let config = SearchConfig {
            limit: 10,
            ..SearchConfig::default()
        };
        let params = SearchParams {
            query: "test".to_owned(),
            workflow: None,
            after: None,
            before: None,
            output_format: "markdown".to_owned(),
            limit_per_domain: Some(1),
            cache: true,
        };
        let ctx = fake_request_context().await;

        let result = search_handler(&mock, params, &ctx, &config, None).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert_eq!(text, "No results found.");
    }

    async fn fake_request_context() -> RequestContext<RoleServer> {
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
}
