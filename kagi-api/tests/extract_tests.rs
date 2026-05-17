use kagi_api::{ExtractPage, ExtractRequest, KagiClientBuilder};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

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
