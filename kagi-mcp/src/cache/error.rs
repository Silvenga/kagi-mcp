use std::io;

/// Error type for cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// I/O error (e.g., directory creation failure).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// SQLite error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    /// Invalid configuration.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}
