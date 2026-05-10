use kagi_api::client::KagiClientBuilder;
use kagi_mcp::server::KagiMcpServer;
use kagi_mcp::validation::validate_extract_urls;
use rmcp::ServerHandler;

#[test]
fn when_initialize_then_returns_server_name_kagi() {
    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .build()
        .unwrap();

    let server = KagiMcpServer::new(client);

    let info = server.get_info();

    assert_eq!(info.server_info.name, "Kagi");
}

#[test]
fn when_list_tools_then_returns_search_and_extract() {
    let client = KagiClientBuilder::new()
        .api_key("test-key")
        .build()
        .unwrap();

    let _server = KagiMcpServer::new(client);

    let tools = KagiMcpServer::tool_router().list_all();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        names.contains(&"search"),
        "tools should contain 'search', got: {names:?}"
    );
    assert!(
        names.contains(&"extract"),
        "tools should contain 'extract', got: {names:?}"
    );
    assert_eq!(tools.len(), 2, "expected exactly 2 tools, got: {names:?}");
}

#[test]
fn when_extract_with_private_ip_then_validation_returns_error() {
    let urls = vec!["https://192.168.1.1/".to_string()];

    let result = validate_extract_urls(&urls);

    assert!(result.is_err(), "private IP should be rejected");
}

#[test]
fn when_extract_with_valid_urls_then_validation_passes() {
    let urls = vec![
        "https://example.com".to_string(),
        "https://kagi.com/api".to_string(),
    ];

    let result = validate_extract_urls(&urls);

    assert!(result.is_ok(), "valid URLs should be accepted");
}
