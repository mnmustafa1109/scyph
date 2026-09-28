//! RAII WebSocket connection session handle.

use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::registry::{ConnectionRegistry, DEFAULT_CONNECTION_BUFFER_SIZE};

/// RAII guard representing an active WebSocket client session registered with [`ConnectionRegistry`].
///
/// Automatically deregisters the connection from the registry when dropped or closed.
#[derive(Debug)]
pub struct RealtimeSession {
    user_id: Uuid,
    conn_id: Uuid,
    rx: Receiver<String>,
    registry: ConnectionRegistry,
    closed: bool,
}

impl RealtimeSession {
    /// Constructs a new [`RealtimeSession`] with default channel buffer capacity (256) and registers it with the given [`ConnectionRegistry`].
    pub async fn connect(registry: ConnectionRegistry, user_id: Uuid) -> Self {
        Self::connect_with_capacity(registry, user_id, DEFAULT_CONNECTION_BUFFER_SIZE).await
    }

    /// Constructs a new [`RealtimeSession`] with custom channel buffer capacity and registers it with the given [`ConnectionRegistry`].
    pub async fn connect_with_capacity(
        registry: ConnectionRegistry,
        user_id: Uuid,
        buffer_size: usize,
    ) -> Self {
        let conn_id = Uuid::now_v7();
        let (tx, rx) = tokio::sync::mpsc::channel(buffer_size.max(1));
        registry.register(user_id, conn_id, tx).await;
        Self {
            user_id,
            conn_id,
            rx,
            registry,
            closed: false,
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
    pub async fn close(mut self) {
        self.closed = true;
        self.registry.deregister(self.user_id, self.conn_id).await;
    }
}

impl Drop for RealtimeSession {
    fn drop(&mut self) {
        if !self.closed {
            let registry = self.registry.clone();
            let user_id = self.user_id;
            let conn_id = self.conn_id;
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    registry.deregister(user_id, conn_id).await;
                });
            }
        }
    }
}
