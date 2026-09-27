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
}

impl From<IdempotencyError> for AppError {
    fn from(err: IdempotencyError) -> Self {
        match err {
            IdempotencyError::Redis(e) => AppError::internal_from(e, "Redis operation failed"),
            IdempotencyError::Json(e) => AppError::internal_from(e, "JSON operation failed"),
        }
    }
}
