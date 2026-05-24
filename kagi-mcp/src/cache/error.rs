use sqlx::migrate::MigrateError;
use std::io;

/// Error type for cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// I/O error (e.g., directory creation failure).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// SQLx error.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Migration error.
    #[error("migration error: {0}")]
    Migrate(#[from] MigrateError),

    /// JSON serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    /// Invalid configuration.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// Corrupt cache entry (e.g., CID is not 16 bytes).
    #[error("corrupt cache entry: {0}")]
    CorruptEntry(String),
}
