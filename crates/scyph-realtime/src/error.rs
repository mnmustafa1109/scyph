//! Realtime WebSocket broadcaster error types.

use scyph_core::AppError;

/// Error type for real-time Redis pub/sub and WebSocket broadcasting operations.
#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    /// Redis client, connection, or pub/sub operation error.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Event JSON serialization or deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Missing or invalid configuration parameter or environment variable.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection channel delivery failure (e.g., channel closed).
    #[error("Delivery error: {0}")]
    Delivery(String),

    /// Generic internal real-time error.
    #[error("Internal realtime error: {0}")]
    Internal(String),
}

impl From<RealtimeError> for AppError {
    /// Converts a [`RealtimeError`] into an RFC 7807 [`AppError`] response.
    fn from(err: RealtimeError) -> Self {
        AppError::internal(err.to_string())
    }
}
