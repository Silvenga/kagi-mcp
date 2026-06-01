use std::io;

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
