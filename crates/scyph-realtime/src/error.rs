//! Realtime WebSocket broadcaster error types.

use scyph_core::AppError;

/// Error type for real-time Redis pub/sub and WebSocket broadcasting operations.
///
/// `RealtimeError` covers the full lifecycle of realtime operations: configuration loading,
/// Redis connectivity, message serialization, and in-process delivery. It implements
/// `From<RealtimeError> for AppError` for automatic conversion in Axum handlers.
///
/// # Error Mapping
///
/// | Variant | HTTP Status (via `AppError`) | Cause |
/// |---|---|---|
/// | [`Redis`](Self::Redis) | 500 Internal Server Error | Redis connection/pub/sub failure |
/// | [`Serialization`](Self::Serialization) | 500 Internal Server Error | JSON encode/decode failure |
/// | [`Configuration`](Self::Configuration) | 500 Internal Server Error | Missing/invalid env vars |
/// | [`Delivery`](Self::Delivery) | 500 Internal Server Error | MPSC channel closed |
/// | [`Internal`](Self::Internal) | 500 Internal Server Error | Unclassified internal error |
///
/// # Example
///
/// ```rust,ignore
/// use scyph_realtime::RealtimeError;
///
/// async fn send_notification(broadcaster: &RealtimeBroadcaster, user_id: Uuid) -> Result<(), RealtimeError> {
///     broadcaster.publish("notification.created", user_id, &serde_json::json!({
///         "message": "You have a new notification"
///     })).await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    /// Redis client, connection, or pub/sub operation error.
    ///
    /// Occurs when Redis is unreachable, authentication fails, the Pub/Sub subscription
    /// drops, or a PUBLISH command fails. The subscriber loop catches this variant and
    /// automatically retries after `reconnect_interval_secs`.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Event JSON serialization or deserialization failure.
    ///
    /// Occurs when a payload cannot be serialized to JSON for publishing, or when an
    /// incoming Redis message cannot be deserialized into [`RawRealtimeEnvelope`](crate::event::RawRealtimeEnvelope)
    /// or a typed [`RealtimeEvent<T>`](crate::event::RealtimeEvent).
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Missing or invalid configuration parameter or environment variable.
    ///
    /// Occurs when [`RealtimeConfig::from_env`](crate::config::RealtimeConfig::from_env)
    /// encounters an unparseable environment variable (e.g., `REALTIME_RECONNECT_INTERVAL_SECS`
    /// containing a non-numeric value).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection channel delivery failure.
    ///
    /// Occurs when an MPSC channel sender returns `Err(SendError)` because the receiver
    /// has been dropped (i.e., the WebSocket connection closed before the message could
    /// be delivered). This is distinct from a full buffer, which is silently dropped.
    #[error("Delivery error: {0}")]
    Delivery(String),

    /// Generic internal real-time error for unclassified failure paths.
    #[error("Internal realtime error: {0}")]
    Internal(String),
}

impl From<RealtimeError> for AppError {
    /// Converts a [`RealtimeError`] into an RFC 7807 [`AppError`] response.
    ///
    /// All variants map to `500 Internal Server Error` since realtime errors are
    /// infrastructure-level failures not attributable to client input.
    fn from(err: RealtimeError) -> Self {
        AppError::internal(err.to_string())
    }
}
