//! In-memory WebSocket connection registry.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, mpsc::Sender};
use uuid::Uuid;

/// Default buffer capacity for a WebSocket connection channel.
///
/// Each [`RealtimeSession`](crate::session::RealtimeSession) allocates a bounded MPSC channel
/// with this capacity. When the buffer is full and a new message arrives, the message is dropped
/// (not blocked) to prevent slow or unresponsive WebSocket clients from causing memory exhaustion
/// (OOM) or head-of-line blocking across the entire registry.
///
/// To use a custom capacity, use
/// [`RealtimeSession::connect_with_capacity`](crate::session::RealtimeSession::connect_with_capacity).
pub const DEFAULT_CONNECTION_BUFFER_SIZE: usize = 256;

/// Sender handle for an active WebSocket connection instance.
///
/// A cloneable tokio MPSC sender for a bounded `String` channel. One sender is stored per
/// active WebSocket connection in the [`ConnectionRegistry`]. Messages are delivered to the
/// corresponding [`RealtimeSession`](crate::session::RealtimeSession) receiver.
pub type ConnectionTx = Sender<String>;

/// Map of connection IDs to their sender handles for a single user.
///
/// A single user may have multiple concurrent WebSocket connections (e.g., multiple browser
/// tabs or devices). Each connection is tracked by a unique UUIDv7 `conn_id`.
pub type UserConnections = HashMap<Uuid, ConnectionTx>;

/// Thread-safe registry mapping user identifiers to their active connection channels.
///
/// `ConnectionRegistry` is the in-process fan-out layer. When the Redis subscriber loop
/// receives an event targeted at `user_id`, it calls [`broadcast_local`](Self::broadcast_local)
/// to forward the raw JSON string to every active WebSocket connection for that user on
/// this replica.
///
/// ## OOM Protection
///
/// Message delivery uses non-blocking [`try_send`](tokio::sync::mpsc::Sender::try_send). If
/// a connection's channel buffer is full (slow client), the message is **dropped** with a
/// `WARN`-level tracing event rather than blocking or accumulating unbounded memory.
///
/// ## Automatic Cleanup
///
/// [`deregister`](Self::deregister) automatically removes empty user entries from the map,
/// so the registry never retains stale `user_id` keys after all connections for that user
/// disconnect.
///
/// ## Cloneability
///
/// `ConnectionRegistry` wraps an `Arc<RwLock<...>>` and is cheap to clone. Pass it to
/// multiple tasks or WebSocket handlers without additional synchronization.
///
/// # Example
///
/// ```rust,ignore
/// use scyph_realtime::ConnectionRegistry;
/// use uuid::Uuid;
///
/// let registry = ConnectionRegistry::default();
///
/// // Typically managed through RealtimeSession, but can be used directly:
/// let (tx, mut rx) = tokio::sync::mpsc::channel(256);
/// let user_id = Uuid::new_v4();
/// let conn_id = Uuid::now_v7();
/// registry.register(user_id, conn_id, tx).await;
///
/// registry.broadcast_local(user_id, r#"{"event":"ping"}"#).await;
///
/// let msg = rx.recv().await.unwrap();
/// assert_eq!(msg, r#"{"event":"ping"}"#);
///
/// registry.deregister(user_id, conn_id).await;
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConnectionRegistry {
    inner: Arc<RwLock<HashMap<Uuid, UserConnections>>>,
}

impl ConnectionRegistry {
    /// Registers a new WebSocket connection for a user.
    ///
    /// Inserts `tx` under `(user_id, conn_id)` in the registry. If no entry exists for
    /// `user_id`, one is created automatically.
    pub async fn register(&self, user_id: Uuid, conn_id: Uuid, tx: ConnectionTx) {
        let mut guard = self.inner.write().await;
        guard.entry(user_id).or_default().insert(conn_id, tx);
    }

    /// Removes a WebSocket connection for a user.
    ///
    /// If this was the last connection for `user_id`, the user entry is also removed from
    /// the registry to prevent unbounded map growth over time.
    pub async fn deregister(&self, user_id: Uuid, conn_id: Uuid) {
        let mut guard = self.inner.write().await;
        if let Some(conns) = guard.get_mut(&user_id) {
            conns.remove(&conn_id);
            if conns.is_empty() {
                guard.remove(&user_id);
            }
        }
    }

    /// Delivers a raw serialized message string to all active connection channels for `user_id` on this replica.
    ///
    /// Uses non-blocking `try_send` for each connection. If a channel buffer is full, the
    /// message is dropped for that connection and a `WARN`-level trace event is emitted.
    /// Other connections for the same user are unaffected.
    pub async fn broadcast_local(&self, user_id: Uuid, raw_message: &str) {
        let guard = self.inner.read().await;
        if let Some(conns) = guard.get(&user_id) {
            for (&conn_id, tx) in conns.iter() {
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) =
                    tx.try_send(raw_message.to_string())
                {
                    tracing::warn!(
                        user_id = %user_id,
                        conn_id = %conn_id,
                        "WebSocket channel buffer full; dropping message to prevent OOM"
                    );
                }
            }
        }
    }

    /// Delivers a raw serialized message string to ALL active connection channels across ALL users connected to this replica.
    ///
    /// Used for global broadcast events (e.g., system announcements, maintenance notices).
    /// Uses non-blocking `try_send` with the same OOM protection as [`broadcast_local`](Self::broadcast_local).
    pub async fn broadcast_global(&self, raw_message: &str) {
        let guard = self.inner.read().await;
        for (&user_id, conns) in guard.iter() {
            for (&conn_id, tx) in conns.iter() {
                if let Err(tokio::sync::mpsc::error::TrySendError::Full(_)) =
                    tx.try_send(raw_message.to_string())
                {
                    tracing::warn!(
                        user_id = %user_id,
                        conn_id = %conn_id,
                        "WebSocket channel buffer full; dropping message to prevent OOM"
                    );
                }
            }
        }
    }

    /// Returns total active connection count across all users on this server replica.
    ///
    /// Useful for metrics, health checks, and capacity planning dashboards.
    pub async fn active_connections_count(&self) -> usize {
        let guard = self.inner.read().await;
        guard.values().map(|c| c.len()).sum()
    }

    /// Returns active connection count for a specific user ID on this server replica.
    ///
    /// Returns `0` if the user has no registered connections.
    pub async fn user_connection_count(&self, user_id: Uuid) -> usize {
        let guard = self.inner.read().await;
        guard.get(&user_id).map(|c| c.len()).unwrap_or(0)
    }
}
