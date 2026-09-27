//! Idempotency error types.

use scyph_core::AppError;

/// Errors encountered during idempotency check and storage operations.
#[derive(Debug, thiserror::Error)]
pub enum IdempotencyError {
    /// Redis database connection or query execution failure.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// JSON serialization or deserialization failure.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// A concurrent request with the same idempotency key is currently in progress.
    #[error("A request with key '{0}' is currently in progress")]
    Conflict(String),
}

impl From<IdempotencyError> for AppError {
    fn from(err: IdempotencyError) -> Self {
        match err {
            IdempotencyError::Conflict(msg) => AppError::Conflict(msg),
            IdempotencyError::Redis(e) => AppError::internal_from(e, "Redis operation failed"),
            IdempotencyError::Json(e) => AppError::internal_from(e, "JSON operation failed"),
        }
    }
}
