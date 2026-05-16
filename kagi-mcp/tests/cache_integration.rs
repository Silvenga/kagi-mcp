use kagi_api::KagiClientBuilder;
use kagi_api::MockKagiApi;
use kagi_api::{Meta, SearchData, SearchRequest, SearchResponse, SearchResult};
use kagi_mcp::cache::key::generate_cache_key;
use kagi_mcp::cache::store::CacheStore;
use kagi_mcp::server::KagiMcpServer;
use kagi_mcp::tools::search::{search_handler, SearchConfig, SearchParams};
use rmcp::model::{ClientInfo, RequestId};
use rmcp::service::{serve_directly_with_ct, RequestContext};
use rmcp::RoleServer;
use tempfile::TempDir;
use tokio::io::duplex;
use tokio_util::sync::CancellationToken;

async fn test_request_context() -> RequestContext<RoleServer> {
    let (server_transport, client_transport) = duplex(4096);
    drop(client_transport);

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .build()
        .expect("KagiClient should build in test");
    let server = KagiMcpServer::new(client, 4.0, 30.0, 10, true, None, None);
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

fn make_search_response(title: &str, snippet: &str) -> SearchResponse {
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

#[tokio::test]
async fn when_cache_persists_across_instances_then_data_should_be_readable() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");

    let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();
    store
        .set("key-persist", "search", b"persistent_payload")
        .unwrap();
    drop(store);

    let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();
    let result = store.get("key-persist").unwrap();

    assert_eq!(result, Some(b"persistent_payload".to_vec()));
}

#[tokio::test]
async fn when_concurrent_readers_then_both_should_read_same_entry() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");

    let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();
    store.set("key-shared", "search", b"shared_data").unwrap();
    drop(store);

    let store_a = CacheStore::new(&cache_dir, 1.0, 7).unwrap();
    let store_b = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

    let result_a = store_a.get("key-shared").unwrap();
    let result_b = store_b.get("key-shared").unwrap();

    assert_eq!(result_a, Some(b"shared_data".to_vec()));
    assert_eq!(result_b, Some(b"shared_data".to_vec()));
}

#[tokio::test]
async fn when_cache_hit_then_api_should_not_be_called() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

    // Pre-populate the cache with a response whose key matches the request
    // that `search_handler` constructs internally.
    let cached_response = make_search_response("Cached Title", "Cached snippet");

    let request = SearchRequest::new("test query")
        .with_format("json".to_owned())
        .with_timeout_seconds(SearchConfig::default().search_timeout)
        .with_limit(SearchConfig::default().limit)
        .with_safe_search(SearchConfig::default().safe_search);
    let cache_key = generate_cache_key(&request);
    store
        .set(
            &cache_key,
            "search",
            &serde_json::to_vec(&cached_response).unwrap(),
        )
        .unwrap();

    // MockKagiApi with NO expectations — calling `.search()` would panic.
    let mock = MockKagiApi::new();

    let ctx = test_request_context().await;
    let params = SearchParams {
        query: "test query".into(),
        workflow: None,
        after: None,
        before: None,
        output_format: None,
        limit_per_domain: None,
        cache: true,
    };

    let result = search_handler(&mock, params, &ctx, &SearchConfig::default(), Some(&store)).await;

    assert!(result.is_ok());
    let text = result.unwrap().content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Cached Title"));
    assert!(text.contains("Cached snippet"));
}
