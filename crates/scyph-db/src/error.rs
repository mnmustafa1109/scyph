//! Database error type definitions.

use scyph_core::AppError;

/// Error type for database operations in `scyph-db`.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// Failure connecting to or querying PostgreSQL via `sqlx`.
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Missing or invalid configuration (e.g. environment variables).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Migration runner failure.
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Seed execution or script error.
    #[error("Seed error: {0}")]
    Seed(String),
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        AppError::internal(err.to_string())
    }
}
