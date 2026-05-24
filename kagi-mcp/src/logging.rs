use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::{Duration, SystemTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::daily;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_RETENTION_DAYS: u64 = 30;

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
    fs::create_dir_all(cache_dir).map_err(|e| {
        anyhow::anyhow!(
            "failed to create cache directory {}: {e}",
            cache_dir.display()
        )
    })?;

    cleanup_old_logs(cache_dir, DEFAULT_LOG_RETENTION_DAYS);

    let appender = daily(cache_dir, "kagi-mcp.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    let pid = process::id();

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(move || PidLineWriter::new(pid, non_blocking.clone()));

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let has_stdout_layer = if is_streamable_http {
        let stdout_layer = fmt::layer()
            .with_ansi(false)
            .with_writer(move || PidLineWriter::new(pid, io::stdout()));
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

fn cleanup_old_logs(log_dir: &Path, max_age_days: u64) {
    let now = SystemTime::now();
    let cutoff = now
        .checked_sub(Duration::from_secs(max_age_days * 86_400))
        .unwrap_or(now);

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("kagi-mcp.log.") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff {
                                if let Err(e) = fs::remove_file(entry.path()) {
                                    tracing::warn!(
                                        path = %entry.path().display(),
                                        error = %e,
                                        "failed to delete old log file"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

struct PidLineWriter<W: Write> {
    prefix: Vec<u8>,
    buf: Vec<u8>,
    inner: W,
}

impl<W: Write> PidLineWriter<W> {
    fn new(pid: u32, inner: W) -> Self {
        Self {
            prefix: format!("[pid={pid}] ").into_bytes(),
            buf: Vec::new(),
            inner,
        }
    }

    fn flush_line(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            self.inner.write_all(&self.prefix)?;
            self.inner.write_all(&self.buf)?;
            self.buf.clear();
        }
        Ok(())
    }
}

impl<W: Write> Write for PidLineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line_end = pos + 1;
            let line = self.buf.split_off(line_end);
            self.inner.write_all(&self.prefix)?;
            self.inner.write_all(&self.buf)?;
            self.buf = line;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_line()?;
        self.inner.flush()
    }
}

impl<W: Write> Drop for PidLineWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush_line();
        let _ = self.inner.flush();
    }
}
