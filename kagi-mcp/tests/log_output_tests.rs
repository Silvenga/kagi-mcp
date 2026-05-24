use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn find_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn spawn_streamable_http(port: u16, mock_uri: &str) -> (std::process::Child, TempDir) {
    let cache_dir = TempDir::new().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("kagi-mcp");
    let mut cmd = Command::new(bin);
    cmd.args([
        "--cache-dir",
        cache_dir.path().to_str().unwrap(),
        "--transport",
        "streamable-http",
        "--bind",
        &format!("127.0.0.1:{}", port),
        "--stateless-json",
    ])
    .env("KAGI_API_KEY", "test-key")
    .env("KAGI_BASE_URL", mock_uri)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let child = cmd.spawn().unwrap();
    (child, cache_dir)
}

async fn wait_for_server(port: u16) {
    for _ in 0..50 {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("Server did not start within timeout");
}

async fn send_mcp_request(client: &reqwest::Client, port: u16, body: &str) -> serde_json::Value {
    let url = format!("http://127.0.0.1:{}/mcp", port);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(body.to_owned())
        .send()
        .await
        .expect("HTTP request should succeed");

    assert_eq!(
        response.status(),
        200,
        "HTTP status should be 200 for body: {body}"
    );

    response
        .json()
        .await
        .expect("response should be valid JSON")
}

fn read_stdout(child: &mut std::process::Child) -> String {
    let mut output = String::new();
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    output
}

const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;

#[tokio::test]
async fn when_search_request_then_logs_contain_query_and_timing() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json")),
        )
        .mount(&mock_server)
        .await;

    let port = find_free_port();
    let (mut child, _cache_dir) = spawn_streamable_http(port, &mock_server.uri());

    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let init_resp = send_mcp_request(&client, port, INIT_BODY).await;
    assert_eq!(
        init_resp["jsonrpc"], "2.0",
        "initialize failed: {init_resp:?}"
    );
    assert_eq!(init_resp["id"], 1, "initialize failed: {init_resp:?}");

    let _search_resp = send_mcp_request(
        &client,
        port,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"query":"test query"}}}"#,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    child.kill().unwrap();
    child.wait().unwrap();

    let stdout = read_stdout(&mut child);

    assert!(
        stdout.contains("test query"),
        "stdout should contain query string\ngot: {stdout}"
    );
    assert!(
        stdout.contains("elapsed_ms="),
        "stdout should contain elapsed_ms=\ngot: {stdout}"
    );
    assert!(
        stdout.contains("cache_hit="),
        "stdout should contain cache_hit=\ngot: {stdout}"
    );
}

#[tokio::test]
async fn when_extract_request_then_logs_contain_url_count_and_timing() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/extract"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/extract-response.json")),
        )
        .mount(&mock_server)
        .await;

    let port = find_free_port();
    let (mut child, _cache_dir) = spawn_streamable_http(port, &mock_server.uri());

    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let init_resp = send_mcp_request(&client, port, INIT_BODY).await;
    assert_eq!(
        init_resp["jsonrpc"], "2.0",
        "initialize failed: {init_resp:?}"
    );
    assert_eq!(init_resp["id"], 1, "initialize failed: {init_resp:?}");

    let _extract_resp = send_mcp_request(
        &client,
        port,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"extract","arguments":{"pages":["https://www.rust-lang.org"]}}}"#,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    child.kill().unwrap();
    child.wait().unwrap();

    let stdout = read_stdout(&mut child);

    assert!(
        stdout.contains("url_count="),
        "stdout should contain url_count=\ngot: {stdout}"
    );
    assert!(
        stdout.contains("elapsed_ms="),
        "stdout should contain elapsed_ms=\ngot: {stdout}"
    );
}

#[tokio::test]
async fn when_cache_hit_then_logs_indicate_cache_hit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json")),
        )
        .mount(&mock_server)
        .await;

    let port = find_free_port();
    let (mut child, _cache_dir) = spawn_streamable_http(port, &mock_server.uri());

    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let init_resp = send_mcp_request(&client, port, INIT_BODY).await;
    assert_eq!(
        init_resp["jsonrpc"], "2.0",
        "initialize failed: {init_resp:?}"
    );
    assert_eq!(init_resp["id"], 1, "initialize failed: {init_resp:?}");

    let _resp1 = send_mcp_request(
        &client,
        port,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"query":"cache test"}}}"#,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let _resp2 = send_mcp_request(
        &client,
        port,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search","arguments":{"query":"cache test"}}}"#,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    child.kill().unwrap();
    child.wait().unwrap();

    let stdout = read_stdout(&mut child);

    assert!(
        stdout.contains("cache_hit=true"),
        "stdout should contain cache_hit=true for second request\ngot: {stdout}"
    );
}

#[tokio::test]
async fn when_cache_miss_then_logs_indicate_cache_miss() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search-response.json")),
        )
        .mount(&mock_server)
        .await;

    let port = find_free_port();
    let (mut child, _cache_dir) = spawn_streamable_http(port, &mock_server.uri());

    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let init_resp = send_mcp_request(&client, port, INIT_BODY).await;
    assert_eq!(
        init_resp["jsonrpc"], "2.0",
        "initialize failed: {init_resp:?}"
    );
    assert_eq!(init_resp["id"], 1, "initialize failed: {init_resp:?}");

    let _resp1 = send_mcp_request(
        &client,
        port,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"query":"first query"}}}"#,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    child.kill().unwrap();
    child.wait().unwrap();

    let stdout = read_stdout(&mut child);

    assert!(
        stdout.contains("cache_hit=false"),
        "stdout should contain cache_hit=false for first request\ngot: {stdout}"
    );
}

#[test]
fn when_stdio_transport_then_no_log_output_on_stdout() {
    let cache_dir = TempDir::new().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("kagi-mcp");
    let mut cmd = Command::new(bin);
    cmd.args([
        "--cache-dir",
        cache_dir.path().to_str().unwrap(),
        "--transport",
        "stdio",
    ])
    .env("KAGI_API_KEY", "test-key")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    std::thread::sleep(Duration::from_millis(500));

    child.kill().unwrap();
    child.wait().unwrap();

    let mut stdout = String::new();
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();

    assert!(
        stdout.is_empty(),
        "stdout should be empty in stdio mode (no MCP messages sent)\ngot: {stdout}"
    );
}
