use common::spawn_server;
use rmcp::model::{CallToolRequestParams, ErrorCode};
use rmcp::service::ServiceError;
use serde_json::json;
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

mod common;

#[tokio::test]
async fn when_initialize_then_returns_server_name_kagi() {
    let mock_server = MockServer::start().await;
    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let info = harness.running.peer().peer_info();
    assert!(info.is_some(), "server info should be present");
    assert_eq!(info.unwrap().server_info.name, "Kagi");

    harness.cleanup().await;
}

#[tokio::test]
async fn when_list_tools_then_returns_search_and_extract() {
    let mock_server = MockServer::start().await;
    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let tools = harness
        .running
        .peer()
        .list_tools(None)
        .await
        .expect("list_tools should succeed");

    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"search"),
        "tools should contain 'search', got: {names:?}"
    );
    assert!(
        names.contains(&"extract"),
        "tools should contain 'extract', got: {names:?}"
    );
    assert_eq!(
        tools.tools.len(),
        2,
        "expected exactly 2 tools, got: {names:?}"
    );

    harness.cleanup().await;
}

#[tokio::test]
async fn when_search_with_valid_query_then_returns_markdown() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json")),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let arguments = json!({"query": "rust"}).as_object().unwrap().clone();
    let params = CallToolRequestParams::new("search").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("search call_tool should succeed");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Rust Programming Language"));
    assert!(text.contains("https://rust-lang.org/"));

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_valid_urls_then_returns_markdown() {
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
async fn when_extract_with_private_ip_then_returns_validation_error() {
    let mock_server = MockServer::start().await;
    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let arguments = json!({"pages": ["https://192.168.1.1/"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness.running.peer().call_tool(params).await;

    match result {
        Err(ServiceError::McpError(err)) => {
            assert!(
                err.to_string().contains("URL validation failed"),
                "error should mention URL validation, got: {err}"
            );
            assert!(
                err.to_string().contains("private IP"),
                "error should mention private IP, got: {err}"
            );
        }
        other => panic!("expected McpError, got: {other:?}"),
    }

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_with_zero_pages_then_returns_invalid_params_error() {
    let mock_server = MockServer::start().await;
    let harness = spawn_server(&mock_server.uri(), &[]).await;

    let arguments = json!({"pages": []}).as_object().unwrap().clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness.running.peer().call_tool(params).await;

    match result {
        Err(ServiceError::McpError(err)) => {
            assert!(
                err.to_string().contains("Pages validation failed"),
                "error should mention Pages validation, got: {err}"
            );
            assert_eq!(
                err.code,
                ErrorCode::INVALID_PARAMS,
                "error code should be INVALID_PARAMS"
            );
        }
        other => panic!("expected McpError, got: {other:?}"),
    }

    harness.cleanup().await;
}

#[tokio::test]
async fn when_search_times_out_then_returns_network_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json"))
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--client-timeout", "0.5", "--retries", "0"],
    )
    .await;

    let arguments = json!({"query": "rust"}).as_object().unwrap().clone();
    let params = CallToolRequestParams::new("search").with_arguments(arguments);

    let result = harness.running.peer().call_tool(params).await;

    match result {
        Err(ServiceError::McpError(err)) => {
            assert!(
                err.to_string().contains("Request failed")
                    || err.to_string().contains("timeout")
                    || err.to_string().contains("error"),
                "error should indicate a network/timeout issue, got: {err}"
            );
        }
        other => panic!("expected McpError for timeout, got: {other:?}"),
    }

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_times_out_then_returns_network_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/extract-response.json"))
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--client-timeout", "0.5", "--retries", "0"],
    )
    .await;

    let arguments = json!({"pages": ["https://www.rust-lang.org"]})
        .as_object()
        .unwrap()
        .clone();
    let params = CallToolRequestParams::new("extract").with_arguments(arguments);

    let result = harness.running.peer().call_tool(params).await;

    match result {
        Err(ServiceError::McpError(err)) => {
            assert!(
                err.to_string().contains("Request failed")
                    || err.to_string().contains("timeout")
                    || err.to_string().contains("error"),
                "error should indicate a network/timeout issue, got: {err}"
            );
        }
        Ok(call_tool_result) => {
            let text = call_tool_result.content[0].as_text().unwrap().text.clone();
            assert!(
                text.contains("Extraction failed") || text.contains("error sending request"),
                "response should contain extraction failure message, got: {text}"
            );
        }
        other => panic!("expected McpError or Ok for timeout, got: {other:?}"),
    }

    harness.cleanup().await;
}

#[tokio::test]
async fn when_search_fails_transiently_then_retries_and_succeeds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "meta": {"trace": "error-trace"},
            "data": null,
            "error": [{"code": "RATE_LIMITED", "url": "", "message": "Too many requests"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json")),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--retries", "1", "--client-timeout", "10.0"],
    )
    .await;

    let arguments = json!({"query": "rust"}).as_object().unwrap().clone();
    let params = CallToolRequestParams::new("search").with_arguments(arguments);

    let result = harness
        .running
        .peer()
        .call_tool(params)
        .await
        .expect("search should succeed after retry");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Rust Programming Language"));

    harness.cleanup().await;
}

#[tokio::test]
async fn when_extract_fails_transiently_then_retries_and_succeeds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "meta": {"trace": "error-trace"},
            "data": null,
            "error": [{"code": "SERVER_ERROR", "url": "", "message": "Internal error"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/extract-response.json")),
        )
        .mount(&mock_server)
        .await;

    let harness = spawn_server(
        &mock_server.uri(),
        &["--retries", "1", "--client-timeout", "10.0"],
    )
    .await;

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
        .expect("extract should succeed after retry");

    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("https://www.rust-lang.org"));
    assert!(text.contains("Performance"));

    harness.cleanup().await;
}
