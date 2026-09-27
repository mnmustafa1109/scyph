//! Realtime event wire envelope protocols and helper deserializer functions.

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::error::RealtimeError;

/// Standardized wire format envelope for WebSocket event broadcasting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeEvent<T> {
    /// Unique event instance identifier (UUIDv7 timestamped).
    pub id: Uuid,
    /// Event topic/name classifier (e.g., `"chat.message"`, `"notification.created"`).
    pub event_name: String,
    /// Target user identifier.
    pub user_id: Uuid,
    /// Event timestamp in UTC.
    pub timestamp: DateTime<Utc>,
    /// Strongly-typed event payload data.
    pub payload: T,
}

impl<T: Serialize> RealtimeEvent<T> {
    /// Constructs a new [`RealtimeEvent`] envelope.
    pub fn new(event_name: impl Into<String>, user_id: Uuid, payload: T) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_name: event_name.into(),
            user_id,
            timestamp: Utc::now(),
            payload,
        }
    }
}

/// Raw untyped event envelope used internally by Redis subscriber for target user routing.
#[derive(Debug, Clone, Deserialize)]
pub struct RawRealtimeEnvelope {
    /// Target user identifier extracted from incoming message payload.
    pub user_id: Uuid,
}

/// Decodes a raw event string into a strongly-typed [`RealtimeEvent<T>`].
///
/// # Errors
///
/// Returns [`RealtimeError::Serialization`] if deserialization fails.
pub fn decode_event<T: DeserializeOwned>(raw: &str) -> Result<RealtimeEvent<T>, RealtimeError> {
    let event: RealtimeEvent<T> = serde_json::from_str(raw)?;
    Ok(event)
}
