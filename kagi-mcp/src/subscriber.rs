use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// The result of building a subscriber — contains the guard and metadata about the layers.
#[derive(Debug)]
pub struct SubscriberLayers {
    /// The non-blocking writer guard that must be kept alive for the duration of logging.
    pub guard: WorkerGuard,
    /// Whether the file layer has ANSI escape codes disabled.
    pub file_layer_ansi_enabled: bool,
    /// Whether a stdout layer was added (true for StreamableHttp, false for Stdio).
    pub has_stdout_layer: bool,
}

/// Build and initialize the tracing subscriber for the given transport mode and cache directory.
///
/// The cache directory is created if it does not exist. Returns an error when the directory
/// cannot be created.
///
/// When `is_streamable_http` is true, a stdout layer is added alongside the file layer.
/// When false, only the file layer is used (appropriate for stdio transport).
pub fn build_subscriber(
    is_streamable_http: bool,
    cache_dir: &Path,
) -> anyhow::Result<SubscriberLayers> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        anyhow::anyhow!(
            "failed to create cache directory {}: {e}",
            cache_dir.display()
        )
    })?;

    let appender = tracing_appender::rolling::daily(cache_dir, "kagi-mcp.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let has_stdout_layer = if is_streamable_http {
        let stdout_layer = fmt::layer().with_ansi(false).with_writer(std::io::stdout);
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .try_init();
        true
    } else {
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .try_init();
        false
    };

    Ok(SubscriberLayers {
        guard,
        file_layer_ansi_enabled: false,
        has_stdout_layer,
    })
}
