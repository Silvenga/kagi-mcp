use kagi_api::{KagiClientBuilder, KagiError, SearchRequest};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

mod common;

fn search_request() -> SearchRequest {
    SearchRequest::new("test query")
}

#[tokio::test]
async fn when_http_400_then_should_return_invalid_request_error() {
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
async fn when_http_401_then_should_return_unauthorized_error() {
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
async fn when_http_403_then_should_return_forbidden_error() {
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
async fn when_http_429_then_should_return_rate_limited_error() {
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
async fn when_http_500_then_should_return_server_error() {
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
