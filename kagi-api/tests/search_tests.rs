use kagi_api::{KagiClientBuilder, KagiError, SearchRequest};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

mod common;

fn search_request() -> SearchRequest {
    SearchRequest::new("test query")
}

#[tokio::test]
async fn when_search_succeeds_then_should_return_results() {
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "test-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Example");
}

#[tokio::test]
async fn when_search_returns_400_then_should_return_invalid_request_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(400).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_401_then_should_return_unauthorized_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(401).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_403_then_should_return_forbidden_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(403).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_429_then_should_return_rate_limited_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_500_then_should_return_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_invalid_json_then_should_return_network_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
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
async fn when_search_returns_429_once_then_retry_should_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(common::error_response_json()))
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(1)
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "retry-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Retry Success");
}

#[tokio::test]
async fn when_search_returns_500_once_then_retry_should_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(common::error_response_json()))
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(1)
        .build()
        .unwrap();

    let response = client.search(search_request()).await.unwrap();
    assert_eq!(response.meta.trace, "retry-trace");
    assert_eq!(response.data.search.unwrap()[0].title, "Retry Success");
}

#[tokio::test]
async fn when_default_user_agent_then_should_send_default_value() {
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .build()
        .unwrap();

    let _ = client.search(search_request()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let req = requests.first().unwrap();
    let user_agent = req
        .headers
        .get("user-agent")
        .expect("user-agent header should be present");
    assert_eq!(
        user_agent.to_str().unwrap(),
        concat!(
            "kagi-api/",
            env!("CARGO_PKG_VERSION"),
            " (github.com/Silvenga/kagi-mcp)"
        )
    );
}

#[tokio::test]
async fn when_custom_user_agent_then_should_send_custom_value() {
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_user_agent("my-custom-agent/1.0")
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
