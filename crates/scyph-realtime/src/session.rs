//! RAII WebSocket connection session handle.

use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::registry::{ConnectionRegistry, DEFAULT_CONNECTION_BUFFER_SIZE};

/// RAII guard representing an active WebSocket client session registered with [`ConnectionRegistry`].
///
/// `RealtimeSession` is the recommended way to manage WebSocket connection lifecycle. It:
/// - Allocates a bounded MPSC channel for incoming messages
/// - Registers the sender end in the [`ConnectionRegistry`] under the given `user_id`
/// - Holds the receiver end so handlers can call [`recv`](Self::recv) to await messages
/// - **Automatically deregisters** from the registry when dropped (RAII pattern)
///
/// ## RAII Deregistration
///
/// When `RealtimeSession` goes out of scope (or `close` is called), the connection sender
/// is removed from the registry. The subscriber loop will no longer attempt to deliver
/// messages to this connection, and the closed channel sender will be cleaned up.
///
/// The `Drop` implementation spawns an async deregistration task using the current Tokio
/// runtime handle. If called outside of a Tokio runtime context, deregistration is skipped
/// (harmless since the MPSC sender will be dropped anyway, causing future `try_send` calls
/// to return `Err(Disconnected)`).
///
/// ## Multiple Sessions Per User
///
/// Multiple `RealtimeSession` instances can coexist for the same `user_id` (e.g., multiple
/// browser tabs). Each session has a unique `conn_id` (UUIDv7). Events published to `user_id`
/// are delivered to all active sessions for that user.
///
/// # Example
///
/// ```rust,ignore
/// use axum::extract::{Path, State, WebSocketUpgrade};
/// use axum::response::IntoResponse;
/// use scyph_realtime::RealtimeBroadcaster;
/// use uuid::Uuid;
///
/// async fn ws_handler(
///     ws: WebSocketUpgrade,
///     Path(user_id): Path<Uuid>,
///     State(broadcaster): State<RealtimeBroadcaster>,
/// ) -> impl IntoResponse {
///     ws.on_upgrade(move |mut socket| async move {
///         let mut session = broadcaster.connect_session(user_id).await;
///
///         while let Some(msg) = session.recv().await {
///             if socket.send(axum::extract::ws::Message::Text(msg.into())).await.is_err() {
///                 break; // Client disconnected; session deregisters on drop
///             }
///         }
///         // `session` drops here -> automatically deregisters from ConnectionRegistry
///     })
/// }
/// ```
#[derive(Debug)]
pub struct RealtimeSession {
    user_id: Uuid,
    conn_id: Uuid,
    rx: Receiver<String>,
    registry: ConnectionRegistry,
    closed: bool,
}

impl RealtimeSession {
    /// Constructs a new [`RealtimeSession`] with default channel buffer capacity ([`DEFAULT_CONNECTION_BUFFER_SIZE`] = 256)
    /// and registers it with the given [`ConnectionRegistry`].
    ///
    /// Returns a session whose receiver will yield JSON event strings as they are broadcast
    /// to `user_id` from any replica in the cluster.
    pub async fn connect(registry: ConnectionRegistry, user_id: Uuid) -> Self {
        Self::connect_with_capacity(registry, user_id, DEFAULT_CONNECTION_BUFFER_SIZE).await
    }

    /// Constructs a new [`RealtimeSession`] with custom channel buffer capacity and registers it with the given [`ConnectionRegistry`].
    ///
    /// Use a larger `buffer_size` for clients expected to receive high-frequency events.
    /// The minimum effective capacity is 1 (values of 0 are coerced to 1).
    ///
    /// # Arguments
    ///
    /// * `registry` — The shared registry to register this connection in.
    /// * `user_id` — The user this session belongs to.
    /// * `buffer_size` — Bounded MPSC channel capacity. Messages are dropped when full.
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
    ///
    /// This is the `user_id` that was passed to [`connect`](Self::connect) or
    /// [`connect_with_capacity`](Self::connect_with_capacity).
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Returns the unique connection identifier for this session.
    ///
    /// Generated as a UUIDv7 at session creation time, providing a time-ordered,
    /// collision-free identifier for this specific connection instance.
    pub fn conn_id(&self) -> Uuid {
        self.conn_id
    }

    /// Receives the next incoming serialized event message forwarded to this session.
    ///
    /// Returns `Some(json_string)` when a message is available, or `None` when the
    /// channel sender has been dropped (connection closed from the registry side).
    ///
    /// This is an async operation that suspends the task until a message arrives.
    pub async fn recv(&mut self) -> Option<String> {
        self.rx.recv().await
    }

    /// Manually closes and deregisters the connection session.
    ///
    /// Marks the session as closed to prevent duplicate deregistration in `Drop`,
    /// then immediately deregisters from the [`ConnectionRegistry`]. Prefer this
    /// over waiting for the session to drop when you need deterministic async cleanup.
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
