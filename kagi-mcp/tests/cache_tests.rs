use kagi_api::{
    KagiClientBuilder, Meta, MockKagiApi, SearchData, SearchRequest, SearchResponse, SearchResult,
};
use kagi_mcp::cache::{CacheStore, SearchCacheKey, SearchCachedResult};
use kagi_mcp::tools::search::{search_handler, SearchConfig, SearchParams};
use kagi_mcp::KagiMcpServer;
use rmcp::model::{ClientInfo, RequestId};
use rmcp::service::{serve_directly_with_ct, RequestContext};
use rmcp::RoleServer;
use tempfile::TempDir;
use tokio::io::duplex;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn when_cache_persists_across_instances_then_data_should_be_readable() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");

    let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();
    let key = SearchCacheKey {
        query: "persistent query".to_owned(),
        workflow: None,
        page: None,
        limit: None,
        safe_search: None,
        region: None,
        filters: None,
        lens_id: None,
        lens: None,
        personalizations: None,
    };
    let result = SearchCachedResult {
        response: SearchResponse {
            meta: Meta {
                trace: "persistent".to_owned(),
                node: None,
                ms: None,
            },
            data: empty_search_data(),
        },
    };
    store.set_search_result(&key, &result).await.unwrap();
    drop(store);

    let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();
    let retrieved = store.get_search_result(&key).await;

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().response.meta.trace, "persistent");
}

#[tokio::test]
async fn when_concurrent_readers_then_both_should_read_same_entry() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");

    let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();
    let key = SearchCacheKey {
        query: "shared query".to_owned(),
        workflow: None,
        page: None,
        limit: None,
        safe_search: None,
        region: None,
        filters: None,
        lens_id: None,
        lens: None,
        personalizations: None,
    };
    let result = SearchCachedResult {
        response: SearchResponse {
            meta: Meta {
                trace: "shared".to_owned(),
                node: None,
                ms: None,
            },
            data: empty_search_data(),
        },
    };
    store.set_search_result(&key, &result).await.unwrap();
    drop(store);

    let store_a = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();
    let store_b = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();

    let result_a = store_a.get_search_result(&key).await;
    let result_b = store_b.get_search_result(&key).await;

    assert!(result_a.is_some());
    assert!(result_b.is_some());
    assert_eq!(result_a.unwrap().response.meta.trace, "shared");
    assert_eq!(result_b.unwrap().response.meta.trace, "shared");
}

#[tokio::test]
async fn when_cache_hit_then_api_should_not_be_called() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();

    let cached_response = fake_search_response("Cached Title", "Cached snippet");
    let request = SearchRequest::new("test query")
        .with_format("json".to_owned())
        .with_timeout_seconds(SearchConfig::default().search_timeout)
        .with_limit(1024)
        .with_safe_search(SearchConfig::default().safe_search);
    let key = SearchCacheKey::from_request(&request);
    let cached_result = SearchCachedResult {
        response: cached_response,
    };
    store.set_search_result(&key, &cached_result).await.unwrap();

    let mock = MockKagiApi::new();

    let ctx = fake_request_context().await;
    let params = SearchParams {
        query: "test query".into(),
        workflow: None,
        after: None,
        before: None,
        output_format: "markdown".to_owned(),
        limit_per_domain: None,
        cache: true,
    };

    let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), Some(&store)).await;

    assert!(result.is_ok());
    let text = result.unwrap().content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Cached Title"));
    assert!(text.contains("Cached snippet"));
}

async fn fake_request_context() -> RequestContext<RoleServer> {
    let (server_transport, client_transport) = duplex(4096);
    drop(client_transport);

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .build()
        .expect("KagiClient should build in test");
    let server = KagiMcpServer::new(client)
        .with_search_timeout(4.0)
        .with_extract_timeout(30.0);
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

fn fake_search_response(title: &str, snippet: &str) -> SearchResponse {
    SearchResponse {
        meta: Meta {
            trace: "cache-integration-test".into(),
            node: None,
            ms: None,
        },
        data: SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com/".into(),
                title: title.into(),
                snippet: Some(snippet.into()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        },
    }
}
