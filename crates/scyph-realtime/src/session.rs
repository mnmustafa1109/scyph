//! RAII WebSocket connection session handle.

use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::registry::ConnectionRegistry;

/// RAII guard representing an active WebSocket client session registered with [`ConnectionRegistry`].
///
/// Automatically deregisters the connection from the registry when dropped or closed.
#[derive(Debug)]
pub struct RealtimeSession {
    user_id: Uuid,
    conn_id: Uuid,
    rx: UnboundedReceiver<String>,
    registry: ConnectionRegistry,
}

impl RealtimeSession {
    /// Constructs a new [`RealtimeSession`] and registers it with the given [`ConnectionRegistry`].
    pub async fn connect(registry: ConnectionRegistry, user_id: Uuid) -> Self {
        let conn_id = Uuid::new_v4();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        registry.register(user_id, conn_id, tx).await;
        Self {
            user_id,
            conn_id,
            rx,
            registry,
        }
    }

    /// Returns the target user identifier for this session.
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Returns the unique connection identifier for this session.
    pub fn conn_id(&self) -> Uuid {
        self.conn_id
    }

    /// Receives the next incoming serialized event message forwarded to this session.
    pub async fn recv(&mut self) -> Option<String> {
        self.rx.recv().await
    }

    /// Manually closes and deregisters the connection session synchronously.
    pub async fn close(self) {
        self.registry.deregister(self.user_id, self.conn_id).await;
    }
}

impl Drop for RealtimeSession {
    fn drop(&mut self) {
        let registry = self.registry.clone();
        let user_id = self.user_id;
        let conn_id = self.conn_id;
        tokio::spawn(async move {
            registry.deregister(user_id, conn_id).await;
        });
    }
}
