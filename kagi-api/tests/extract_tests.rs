use kagi_api::types::{ExtractPage, ExtractRequest};
use kagi_api::{KagiClientBuilder, KagiError};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

mod common;

fn extract_request() -> ExtractRequest {
    ExtractRequest::new(vec![ExtractPage {
        url: "https://example.com".to_owned(),
    }])
}

#[tokio::test]
async fn when_extract_succeeds_then_should_return_content() {
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
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .build()
        .unwrap();

    let response = client.extract(extract_request()).await.unwrap();
    assert_eq!(response.meta.trace, "test-trace");
    assert_eq!(
        response.data.unwrap()[0].markdown,
        Some("# Hello".to_owned())
    );
}

#[tokio::test]
async fn when_extract_returns_400_then_should_return_invalid_request_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(400).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
        .build()
        .unwrap();

    let result = client.extract(extract_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::InvalidRequest { .. }),
        "expected InvalidRequest, got {err}"
    );
    assert_eq!(err.to_string(), "invalid request: Error message");
}

#[tokio::test]
async fn when_extract_returns_401_then_should_return_unauthorized_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(401).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
        .build()
        .unwrap();

    let result = client.extract(extract_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::Unauthorized),
        "expected Unauthorized, got {err}"
    );
}

#[tokio::test]
async fn when_extract_returns_403_then_should_return_forbidden_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(403).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
        .build()
        .unwrap();

    let result = client.extract(extract_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::Forbidden),
        "expected Forbidden, got {err}"
    );
}

#[tokio::test]
async fn when_extract_returns_429_then_should_return_rate_limited_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(429).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
        .build()
        .unwrap();

    let result = client.extract(extract_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::RateLimited),
        "expected RateLimited, got {err}"
    );
}

#[tokio::test]
async fn when_extract_returns_500_then_should_return_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(500).set_body_json(common::error_response_json()))
        .mount(&server)
        .await;

    let client = KagiClientBuilder::new()
        .with_api_key("test-key")
        .with_base_url(server.uri())
        .with_retries(0)
        .build()
        .unwrap();

    let result = client.extract(extract_request()).await;
    let err = result.unwrap_err();
    assert!(
        matches!(err, KagiError::ServerError),
        "expected ServerError, got {err}"
    );
}
