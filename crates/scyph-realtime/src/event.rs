//! Realtime event wire envelope protocols and helper deserializer functions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::error::RealtimeError;

/// Trait binding an event name string constant to a strongly-typed payload structure.
///
/// Implement this trait on domain event structs to associate them with a canonical topic string.
/// This enables type-safe publish calls via [`RealtimeBroadcaster::publish_payload`](crate::broadcaster::RealtimeBroadcaster::publish_payload)
/// that infer the event name from the type rather than a magic string argument.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
/// use scyph_realtime::RealtimePayload;
///
/// #[derive(Serialize)]
/// pub struct ChatMessageSent {
///     pub room_id: String,
///     pub text: String,
/// }
///
/// impl RealtimePayload for ChatMessageSent {
///     const EVENT_NAME: &'static str = "chat.message_sent";
/// }
///
/// #[derive(Serialize)]
/// pub struct UserStatusChanged {
///     pub status: String,
/// }
///
/// impl RealtimePayload for UserStatusChanged {
///     const EVENT_NAME: &'static str = "user.status_changed";
/// }
/// ```
pub trait RealtimePayload: Serialize {
    /// The unique event name/topic identifier for this payload type.
    ///
    /// Use a namespaced dotted notation (e.g. `"chat.message"`, `"user.updated"`,
    /// `"notification.created"`) for consistent event taxonomy across clients and services.
    const EVENT_NAME: &'static str;
}

/// Standardized wire format envelope for WebSocket event broadcasting.
///
/// `RealtimeEvent<T>` is the JSON object transmitted over Redis Pub/Sub and forwarded to
/// WebSocket clients. Clients receive the full serialized envelope and can use `event_name`
/// to route to the appropriate handler.
///
/// ## Wire Format
///
/// ```json
/// {
///   "id": "018f2d5e-4a6c-7000-8000-000000000001",
///   "event_name": "chat.message",
///   "user_id": "550e8400-e29b-41d4-a716-446655440000",
///   "is_global": false,
///   "timestamp": "2024-01-15T10:30:00.000Z",
///   "payload": { "text": "Hello!" }
/// }
/// ```
///
/// For global broadcasts, `user_id` is `null` and `is_global` is `true`.
///
/// ## Example: Constructing Events
///
/// ```rust
/// use scyph_realtime::RealtimeEvent;
/// use uuid::Uuid;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Ping { message: String }
///
/// let user_id = Uuid::new_v4();
/// let event = RealtimeEvent::new("ping", user_id, Ping { message: "hello".to_string() });
/// assert_eq!(event.event_name, "ping");
/// assert_eq!(event.user_id, Some(user_id));
/// assert!(!event.is_global);
///
/// let global_event = RealtimeEvent::new_global("maintenance", Ping { message: "down for maintenance".to_string() });
/// assert!(global_event.is_global);
/// assert!(global_event.user_id.is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeEvent<T> {
    /// Unique event instance identifier (UUIDv7 timestamped).
    ///
    /// Generated with [`Uuid::now_v7`] at construction time, providing monotonically ordered
    /// event IDs that clients can use for deduplication or ordering.
    pub id: Uuid,

    /// Event topic/name classifier (e.g., `"chat.message"`, `"notification.created"`).
    ///
    /// Used by WebSocket clients to route received events to the appropriate handler.
    pub event_name: String,

    /// Target user identifier.
    ///
    /// `Some(user_id)` for user-targeted events. `None` for global broadcasts where
    /// `is_global` is `true`.
    pub user_id: Option<Uuid>,

    /// Indicates whether this event is broadcast globally to all connected users.
    ///
    /// When `true`, the subscriber loop calls
    /// [`broadcast_global`](crate::registry::ConnectionRegistry::broadcast_global) instead of
    /// [`broadcast_local`](crate::registry::ConnectionRegistry::broadcast_local).
    #[serde(default)]
    pub is_global: bool,

    /// Event timestamp in UTC at the time of creation.
    pub timestamp: DateTime<Utc>,

    /// Strongly-typed event payload data embedded in the envelope.
    pub payload: T,
}

impl<T: Serialize> RealtimeEvent<T> {
    /// Constructs a new targeted [`RealtimeEvent`] envelope for a single recipient user.
    ///
    /// Sets `is_global = false` and `user_id = Some(user_id)`. The event will be delivered
    /// only to active connections registered for `user_id` on any replica.
    pub fn new(event_name: impl Into<String>, user_id: Uuid, payload: T) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_name: event_name.into(),
            user_id: Some(user_id),
            is_global: false,
            timestamp: Utc::now(),
            payload,
        }
    }

    /// Constructs a new global [`RealtimeEvent`] envelope targetable to all connected users.
    ///
    /// Sets `is_global = true` and `user_id = None`. The event will be delivered to every
    /// active connection on every replica.
    pub fn new_global(event_name: impl Into<String>, payload: T) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_name: event_name.into(),
            user_id: None,
            is_global: true,
            timestamp: Utc::now(),
            payload,
        }
    }
}

/// Raw untyped event envelope used internally by the Redis subscriber for routing.
///
/// When the subscriber loop receives a raw JSON string from Redis, it first deserializes it
/// into this lightweight struct to extract routing metadata (`user_id`, `is_global`) without
/// needing to know the concrete payload type `T`. The original raw string is then forwarded
/// as-is to the appropriate [`ConnectionRegistry`](crate::registry::ConnectionRegistry) channels.
///
/// WebSocket clients receive the raw string and are responsible for deserializing the
/// full [`RealtimeEvent<T>`] on their end.
#[derive(Debug, Clone, Deserialize)]
pub struct RawRealtimeEnvelope {
    /// Target user identifier extracted from incoming message payload.
    ///
    /// Used to look up the appropriate [`UserConnections`](crate::registry::UserConnections)
    /// entry in the registry for local fanout.
    pub user_id: Option<Uuid>,

    /// Indicates whether message is targeted globally to all connected users.
    ///
    /// When `true`, the raw message is delivered to all users via
    /// [`broadcast_global`](crate::registry::ConnectionRegistry::broadcast_global).
    #[serde(default)]
    pub is_global: bool,
}

/// Decodes a raw event string into a strongly-typed [`RealtimeEvent<T>`].
///
/// Typically used on the **client side** of a WebSocket connection to deserialize
/// the raw JSON string received from the server into a typed event envelope.
///
/// # Arguments
///
/// * `raw` — A raw JSON string representing a serialized [`RealtimeEvent<T>`].
///
/// # Errors
///
/// Returns [`RealtimeError::Serialization`] if the string is not valid JSON or if
/// the JSON structure does not match [`RealtimeEvent<T>`].
///
/// # Example
///
/// ```rust,ignore
/// use serde::Deserialize;
/// use scyph_realtime::{RealtimeEvent, decode_event};
///
/// #[derive(Deserialize)]
/// struct ChatPayload { text: String }
///
/// let raw = r#"{"id":"...","event_name":"chat.message","user_id":"...","is_global":false,"timestamp":"...","payload":{"text":"Hi!"}}"#;
/// let event: RealtimeEvent<ChatPayload> = decode_event(raw)?;
/// println!("Received: {}", event.payload.text);
/// ```
pub fn decode_event<T: DeserializeOwned>(raw: &str) -> Result<RealtimeEvent<T>, RealtimeError> {
    let event: RealtimeEvent<T> = serde_json::from_str(raw)?;
    Ok(event)
}
