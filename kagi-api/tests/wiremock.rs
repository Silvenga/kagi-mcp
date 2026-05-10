use kagi_api::{
    KagiClientBuilder, KagiError,
    types::{ExtractPage, ExtractRequest, SearchRequest},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn search_request() -> SearchRequest {
    SearchRequest {
        query: "test query".to_string(),
        workflow: None,
        format: None,
        timeout: None,
        page: None,
        limit: None,
        safe_search: None,
        region: None,
        filters: None,
    }
}

fn extract_request() -> ExtractRequest {
    ExtractRequest {
        pages: vec![ExtractPage {
            url: "https://example.com".to_string(),
        }],
        timeout: None,
        format: None,
    }
}

fn error_response_json() -> serde_json::Value {
    serde_json::json!({
        "meta": { "trace": "error-trace" },
        "data": null,
        "error": [
            { "code": "ERR_CODE", "url": "", "message": "Error message", "location": null }
        ]
    })
}

#[tokio::test]
async fn search_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({ "query": "test query" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "test-trace", "node": "test-node", "ms": 100 },
            "data": {
                "search": [
                    { "url": "https://example.com", "title": "Example" }
                ]
            }
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "test-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Example");
}

#[tokio::test]
async fn extract_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({
            "pages": [{ "url": "https://example.com" }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "test-trace", "node": "test-node", "ms": 100 },
            "data": [
                { "url": "https://example.com", "markdown": "# Hello" }
            ]
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .build()
        .unwrap();

    let response = client.extract(extract_request()).await.unwrap();
    assert_eq!(response.meta.trace, "test-trace");
    assert_eq!(response.data.unwrap()[0].markdown, Some("# Hello".to_string()));
}

#[tokio::test]
async fn error_400() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(400).set_body_json(error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::InvalidRequest { .. }),
        "expected InvalidRequest, got {err}"
    );
    assert_eq!(err.to_string(), "invalid request: Error message");
}

#[tokio::test]
async fn error_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(401).set_body_json(error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::Unauthorized),
        "expected Unauthorized, got {err}"
    );
}

#[tokio::test]
async fn error_403() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(403).set_body_json(error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::Forbidden),
        "expected Forbidden, got {err}"
    );
}

#[tokio::test]
async fn error_429() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::RateLimited),
        "expected RateLimited, got {err}"
    );
}

#[tokio::test]
async fn error_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::ServerError),
        "expected ServerError, got {err}"
    );
}

#[tokio::test]
async fn network_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(0)
        .build()
        .unwrap();

    let result = client.search(search_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::Network { .. }),
        "expected Network, got {err}"
    );
}

#[tokio::test]
async fn retry_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(error_response_json()))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "retry-trace", "node": "test-node", "ms": 100 },
            "data": {
                "search": [
                    { "url": "https://example.com", "title": "Retry Success" }
                ]
            }
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(1)
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "retry-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Retry Success");
}

#[tokio::test]
async fn retry_on_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error_response_json()))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "retry-trace", "node": "test-node", "ms": 100 },
            "data": {
                "search": [
                    { "url": "https://example.com", "title": "Retry Success" }
                ]
            }
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .retries(1)
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "retry-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Retry Success");
}

#[tokio::test]
async fn user_agent_default() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "ua-trace", "node": "test-node", "ms": 100 },
            "data": { "search": [{ "url": "https://example.com", "title": "UA Test" }] }
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .build()
        .unwrap();

    let _ = client.search(search_request()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let req = requests.first().unwrap();
    let user_agent = req
        .headers
        .get("user-agent")
        .expect("user-agent header should be present");
    assert_eq!(user_agent.to_str().unwrap(), "kagi-api/0.1.0 (github.com/Silvenga/kagi-mcp)");
}

#[tokio::test]
async fn user_agent_custom() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "trace": "ua-trace", "node": "test-node", "ms": 100 },
            "data": { "search": [{ "url": "https://example.com", "title": "UA Test" }] }
        })))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .base_url(&server.uri())
        .user_agent("my-custom-agent/1.0")
        .build()
        .unwrap();

    let _ = client.search(search_request()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let req = requests.first().unwrap();
    let user_agent = req
        .headers
        .get("user-agent")
        .expect("user-agent header should be present");
    assert_eq!(user_agent.to_str().unwrap(), "my-custom-agent/1.0");
}
