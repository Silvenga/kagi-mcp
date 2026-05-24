use kagi_mcp::subscriber::build_subscriber;
use tempfile::TempDir;

#[test]
fn when_stdio_transport_then_subscriber_has_file_layer_only() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("kagi-mcp");

    let layers = build_subscriber(false, &cache_dir).unwrap();

    assert!(
        !layers.has_stdout_layer,
        "Stdio transport should not have a stdout layer"
    );
    assert!(
        !layers.file_layer_ansi_enabled,
        "File layer should have ANSI disabled"
    );
}

#[test]
fn when_streamablehttp_transport_then_subscriber_has_file_and_stdout_layers() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("kagi-mcp");

    let layers = build_subscriber(true, &cache_dir).unwrap();

    assert!(
        layers.has_stdout_layer,
        "StreamableHttp transport should have a stdout layer"
    );
    assert!(
        !layers.file_layer_ansi_enabled,
        "File layer should have ANSI disabled"
    );
}

#[test]
fn when_ansi_disabled_then_file_layer_has_no_ansi() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("kagi-mcp");

    let layers = build_subscriber(false, &cache_dir).unwrap();

    assert!(
        !layers.file_layer_ansi_enabled,
        "File layer should have ANSI escape codes disabled"
    );
}

#[test]
fn when_cache_dir_missing_then_startup_fails() {
    let tmp = TempDir::new().unwrap();
    let bad_path = tmp.path().join("not_a_dir");
    std::fs::write(&bad_path, "i am a file, not a directory").unwrap();

    let result = build_subscriber(false, &bad_path);

    assert!(
        result.is_err(),
        "build_subscriber should fail when cache directory cannot be created"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("failed to create cache directory"),
        "Error message should mention cache directory creation failure: {err}"
    );
}
