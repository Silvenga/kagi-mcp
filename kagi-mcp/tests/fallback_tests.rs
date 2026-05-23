use common::{spawn_server, spawn_server_with_env};
use rmcp::model::CallToolRequestParams;
use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

mod common;

#[tokio::test]
async fn when_extract_with_fallback_message_then_empty_content_shows_fallback() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {"trace": "test-trace", "ms": 100, "node": "test-node"},
            "data": [{"url": "https://github.com/some/repo", "markdown": null}],
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--fallback-message", "github.com=Use github-mcp instead"],
    )
    .await;

    let arguments = json!({"pages": ["https://github.com/some/repo"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("Use github-mcp instead"),
        "response should contain fallback message, got: {text}"
    );

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_always_block_then_no_api_call_for_blocked_domain() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {"trace": "test-trace", "ms": 100, "node": "test-node"},
            "data": [{"url": "https://other.com/page", "markdown": "Other content"}],
            "errors": []
        })))
        .expect(0)
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &[
            "--fallback-always",
            "github.com",
            "--fallback-message",
            "github.com=Use github-mcp instead",
        ],
    )
    .await;

    let arguments = json!({"pages": ["https://github.com/some/repo"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("Use github-mcp instead"),
        "response should contain fallback message, got: {text}"
    );

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_no_matching_fallback_then_normal_behavior() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {"trace": "test-trace", "ms": 100, "node": "test-node"},
            "data": [{"url": "https://example.com/page", "markdown": "Normal content"}],
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--fallback-message", "github.com=Use github-mcp instead"],
    )
    .await;

    let arguments = json!({"pages": ["https://example.com/page"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("Normal content"),
        "response should contain normal content, got: {text}"
    );
    assert!(
        !text.contains("Use github-mcp instead"),
        "response should NOT contain fallback message for different domain, got: {text}"
    );

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_env_var_fallback_then_behaves_like_cli() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {"trace": "test-trace", "ms": 100, "node": "test-node"},
            "data": [{"url": "https://github.com/some/repo", "markdown": null}],
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let harness = spawn_server_with_env(
        &mock_server.uri(),
        &[],
        &[("KAGI_FALLBACK_MESSAGE", "github.com=Use github-mcp instead")],
    )
    .await;

    let arguments = json!({"pages": ["https://github.com/some/repo"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("Use github-mcp instead"),
        "response should contain fallback message from env var, got: {text}"
    );

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_without_fallback_config_then_identical_to_before() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/extract-response.json")),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let arguments = json!({"pages": ["https://www.rust-lang.org"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("https://www.rust-lang.org"));
    assert!(text.contains("Performance"));
    assert!(text.contains("Reliability"));

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_multiple_fallback_rules_then_correct_message_for_each_domain() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {"trace": "test-trace", "ms": 100, "node": "test-node"},
            "data": [
                {"url": "https://github.com/some/repo", "markdown": null},
                {"url": "https://gitlab.com/some/repo", "markdown": null}
            ],
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &[
            "--fallback-message",
            "github.com=Use github-mcp instead",
            "--fallback-message",
            "gitlab.com=Use gitlab-mcp instead",
        ],
    )
    .await;

    let arguments =
        json!({"pages": ["https://github.com/some/repo", "https://gitlab.com/some/repo"]})
            .as_object()
            .unwrap()
            .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("extract call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("Use github-mcp instead"),
        "response should contain github fallback message, got: {text}"
    );
    assert!(
        text.contains("Use gitlab-mcp instead"),
        "response should contain gitlab fallback message, got: {text}"
    );

    harness.cleanup().await;
}
