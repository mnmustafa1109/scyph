//! Idempotency error types.
//!
//! Defines [`IdempotencyError`] — failures that can occur during idempotency key reservation,
//! response caching, and cache retrieval operations.

use scyph_core::AppError;

/// Errors encountered during idempotency check and storage operations.
///
/// ## HTTP Mapping
///
/// | Variant | HTTP Status | Meaning |
/// |---------|-------------|---------|
/// | `Redis` | 500 | Infrastructure failure — Redis unavailable or query error |
/// | `Json` | 500 | Serialization bug — cached value is malformed |
/// | `Conflict` | 409 | Duplicate in-flight request with the same idempotency key |
///
/// ## Conflict Handling
///
/// A [`IdempotencyError::Conflict`] response means the exact same request (by idempotency key)
/// is currently being processed by another handler invocation. The client should wait briefly
/// and poll or retry. This is a normal operational response, not an infrastructure failure.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::idempotency::IdempotencyError;
///
/// let err = IdempotencyError::Conflict("key_abc123 is currently in progress".into());
/// assert!(err.to_string().contains("currently in progress"));
/// ```
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
