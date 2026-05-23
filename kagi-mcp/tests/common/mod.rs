use assert_cmd::cargo::cargo_bin;
use rmcp::serve_client;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

pub struct TestHarness {
    pub running: RunningService<RoleClient, ()>,
    child: Child,
    _stderr_handle: JoinHandle<()>,
    _cache_dir: TempDir,
}

impl TestHarness {
    pub async fn cleanup(mut self) {
        let _ = self.running.cancel().await;
        let _ = self.child.kill().await;
    }
}

pub async fn spawn_server(base_url: &str, extra_args: &[&str]) -> TestHarness {
    spawn_server_with_env(base_url, extra_args, &[]).await
}

pub async fn spawn_server_with_env(
    base_url: &str,
    extra_args: &[&str],
    extra_env: &[(&str, &str)],
) -> TestHarness {
    let bin = cargo_bin("kagi-mcp");
    let cache_dir = TempDir::new().expect("failed to create temp cache dir");
    let cache_dir_path = cache_dir.path().to_str().expect("non-utf8 path").to_owned();

    let mut cmd = Command::new(bin);
    cmd.args(["--cache-dir", &cache_dir_path])
        .args(extra_args)
        .env("KAGI_API_KEY", "test-key")
        .env("KAGI_BASE_URL", base_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn kagi-mcp: {e}"));

    let stdin = child.stdin.take().expect("stdin not available");
    let stdout = child.stdout.take().expect("stdout not available");
    let stderr = child.stderr.take().expect("stderr not available");

    let _stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(_line)) = lines.next_line().await {}
    });

    let transport = (stdout, stdin);
    let running = serve_client((), transport)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize MCP client: {e}"));

    TestHarness {
        running,
        child,
        _stderr_handle,
        _cache_dir: cache_dir,
    }
}
