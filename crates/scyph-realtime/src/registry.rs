//! In-memory WebSocket connection registry.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use uuid::Uuid;

/// Sender handle for an active WebSocket connection instance.
pub type ConnectionTx = UnboundedSender<String>;

/// Map of connection IDs to their sender handles for a single user.
pub type UserConnections = HashMap<Uuid, ConnectionTx>;

/// Thread-safe registry mapping user identifiers to their active connection channels.
#[derive(Debug, Clone, Default)]
pub struct ConnectionRegistry {
    inner: Arc<RwLock<HashMap<Uuid, UserConnections>>>,
}

impl ConnectionRegistry {
    /// Registers a new WebSocket connection for a user.
    pub async fn register(&self, user_id: Uuid, conn_id: Uuid, tx: ConnectionTx) {
        let mut guard = self.inner.write().await;
        guard.entry(user_id).or_default().insert(conn_id, tx);
    }

    /// Removes a WebSocket connection for a user. Automatically cleans up empty user maps to prevent memory leaks.
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
    pub async fn broadcast_local(&self, user_id: Uuid, raw_message: &str) {
        let guard = self.inner.read().await;
        if let Some(conns) = guard.get(&user_id) {
            for tx in conns.values() {
                let _ = tx.send(raw_message.to_string());
            }
        }
    }

    /// Returns total active connection count across all users on this server replica.
    pub async fn active_connections_count(&self) -> usize {
        let guard = self.inner.read().await;
        guard.values().map(|c| c.len()).sum()
    }

    /// Returns active connection count for a specific user ID on this server replica.
    pub async fn user_connection_count(&self, user_id: Uuid) -> usize {
        let guard = self.inner.read().await;
        guard.get(&user_id).map(|c| c.len()).unwrap_or(0)
    }
}
