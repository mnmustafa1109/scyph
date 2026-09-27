//! Strongly-typed utility error definitions and RFC 7807 problem details conversions.

use scyph_core::AppError;

/// Utility errors encountered during idempotency checks, webhook verification, pagination, or worker loop execution.
#[derive(Debug, thiserror::Error)]
pub enum UtilsError {
    /// Redis database connection or query execution failure.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Invalid or malformed cursor pagination token.
    #[error("Invalid pagination cursor token: {0}")]
    InvalidCursor(String),

    /// Webhook timestamp is outside permitted tolerance window.
    #[error("Stale webhook timestamp: delta {delta_secs}s exceeds tolerance {tolerance_secs}s")]
    StaleTimestamp {
        /// Configured tolerance threshold in seconds.
        tolerance_secs: i64,
        /// Actual difference between server time and webhook timestamp.
        delta_secs: i64,
    },

    /// HMAC webhook signature verification mismatch.
    #[error("Invalid webhook signature")]
    SignatureMismatch,

    /// Malformed or invalid webhook signature header format.
    #[error("Invalid webhook header format: {0}")]
    InvalidHeaderFormat(String),

    /// Key initialization error for HMAC cryptography.
    #[error("HMAC secret key error: {0}")]
    HmacKey(String),

    /// Error joining Tokio background worker task.
    #[error("Worker task error: {0}")]
    TaskJoin(String),

    /// JSON serialization or deserialization failure.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<UtilsError> for AppError {
    fn from(err: UtilsError) -> Self {
        match err {
            UtilsError::StaleTimestamp { .. }
            | UtilsError::SignatureMismatch
            | UtilsError::InvalidHeaderFormat(_)
            | UtilsError::InvalidCursor(_) => AppError::BadRequest(err.to_string()),
            UtilsError::Redis(e) => AppError::internal_from(e, "Redis operation failed"),
            UtilsError::Json(e) => AppError::internal_from(e, "JSON operation failed"),
            UtilsError::HmacKey(msg) | UtilsError::TaskJoin(msg) => AppError::internal(msg),
        }
    }
}

/// Convenience alias for `Result<T, UtilsError>`.
pub type Result<T, E = UtilsError> = std::result::Result<T, E>;
