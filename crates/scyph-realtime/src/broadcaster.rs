//! Redis pub/sub WebSocket broadcaster implementation.

use futures_util::StreamExt;
use redis::{AsyncCommands, Client};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    config::RealtimeConfig,
    error::RealtimeError,
    event::{RawRealtimeEnvelope, RealtimeEvent, RealtimePayload},
    registry::{ConnectionRegistry, ConnectionTx},
    session::RealtimeSession,
};

/// High-performance Redis Pub/Sub broadcaster for cross-replica WebSocket message fanout.
///
/// `RealtimeBroadcaster` is the central coordination type for real-time messaging. It owns:
/// - A Redis [`Client`] for creating new connections
/// - A [`RealtimeConfig`] with channel prefix and reconnect settings
/// - A shared [`ConnectionRegistry`] tracking all active WebSocket sessions on this replica
/// - A lazily-initialized, internally cached [`redis::aio::MultiplexedConnection`] reused
///   across all publish calls to avoid connection overhead
///
/// `RealtimeBroadcaster` is `Clone` and `Send + Sync`, making it safe to share as Axum
/// [`State`](axum::extract::State) across handlers and tasks.
///
/// # Lifecycle
///
/// 1. Construct with [`new`](Self::new) or [`from_env`](Self::from_env)
/// 2. Call [`start_subscriber`](Self::start_subscriber) to begin consuming Redis Pub/Sub messages
/// 3. Use [`connect_session`](Self::connect_session) in WebSocket handlers to register connections
/// 4. Publish events using [`publish`](Self::publish), [`publish_payload`](Self::publish_payload),
///    [`publish_to_many`](Self::publish_to_many), or [`publish_global`](Self::publish_global)
/// 5. On shutdown, cancel the `CancellationToken` passed to `start_subscriber`
///
/// # Example
///
/// ```rust,ignore
/// use scyph_realtime::{RealtimeBroadcaster, RealtimeConfig};
/// use tokio_util::sync::CancellationToken;
///
/// let broadcaster = RealtimeBroadcaster::from_env()?;
///
/// // Start subscriber loop (runs indefinitely until token is cancelled)
/// let cancel = CancellationToken::new();
/// let _handle = broadcaster.start_subscriber(cancel.clone());
///
/// // Later, on graceful shutdown:
/// cancel.cancel();
/// ```
#[derive(Debug, Clone)]
pub struct RealtimeBroadcaster {
    client: Client,
    config: RealtimeConfig,
    registry: ConnectionRegistry,
    publisher: std::sync::Arc<tokio::sync::Mutex<Option<redis::aio::MultiplexedConnection>>>,
}

impl RealtimeBroadcaster {
    /// Constructs a [`RealtimeBroadcaster`] by reading environment variables using [`RealtimeConfig::from_env`].
    ///
    /// Convenience constructor that combines environment loading and client initialization.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError::Configuration`] if environment variable parsing fails, or
    /// [`RealtimeError::Redis`] if the Redis client cannot be initialized from the parsed URL.
    pub fn from_env() -> Result<Self, RealtimeError> {
        let config = RealtimeConfig::from_env()?;
        Self::new(config)
    }

    /// Constructs a [`RealtimeBroadcaster`] with explicit [`RealtimeConfig`].
    ///
    /// Creates a Redis [`Client`] from `config.redis_url` but does not open a connection yet.
    /// The multiplexed publish connection is created lazily on the first [`publish`](Self::publish) call.
    /// The subscriber connection is created when [`start_subscriber`](Self::start_subscriber) is called.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError::Redis`] if the Redis URL is invalid or the client cannot be created.
    pub fn new(config: RealtimeConfig) -> Result<Self, RealtimeError> {
        let client = Client::open(config.redis_url.as_str())?;
        Ok(Self {
            client,
            config,
            registry: ConnectionRegistry::default(),
            publisher: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Exposes a reference to the inner [`ConnectionRegistry`].
    ///
    /// Useful for inspecting active connection counts or performing manual registration
    /// outside of a [`RealtimeSession`].
    pub fn registry(&self) -> &ConnectionRegistry {
        &self.registry
    }

    /// Exposes a reference to the active [`RealtimeConfig`].
    pub fn config(&self) -> &RealtimeConfig {
        &self.config
    }

    /// Exposes a reference to the underlying Redis [`Client`].
    ///
    /// Can be used to open additional connections (e.g., for caching) without constructing
    /// a separate Redis client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Registers a new active WebSocket connection channel for a target user.
    ///
    /// Prefer [`connect_session`](Self::connect_session) which creates a RAII guard that
    /// automatically deregisters on drop. Use this method only when manual lifecycle control
    /// is required.
    pub async fn register(&self, user_id: Uuid, conn_id: Uuid, tx: ConnectionTx) {
        self.registry.register(user_id, conn_id, tx).await;
    }

    /// Deregisters an active WebSocket connection channel for a target user.
    ///
    /// This is called automatically by [`RealtimeSession`] when it is dropped or closed.
    /// Call this manually only if you used [`register`](Self::register) directly.
    pub async fn deregister(&self, user_id: Uuid, conn_id: Uuid) {
        self.registry.deregister(user_id, conn_id).await;
    }

    /// Creates and registers an RAII WebSocket connection session guard [`RealtimeSession`] for `user_id`.
    ///
    /// Allocates a bounded MPSC channel (capacity [`DEFAULT_CONNECTION_BUFFER_SIZE`](crate::registry::DEFAULT_CONNECTION_BUFFER_SIZE) = 256),
    /// registers the sender end in the [`ConnectionRegistry`], and returns a [`RealtimeSession`]
    /// holding the receiver end.
    ///
    /// The session will automatically deregister from the connection registry when dropped or
    /// explicitly closed via [`RealtimeSession::close`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use axum::extract::{Path, State, WebSocketUpgrade};
    /// use axum::response::IntoResponse;
    /// use uuid::Uuid;
    ///
    /// async fn ws_handler(
    ///     ws: WebSocketUpgrade,
    ///     Path(user_id): Path<Uuid>,
    ///     State(broadcaster): State<RealtimeBroadcaster>,
    /// ) -> impl IntoResponse {
    ///     ws.on_upgrade(move |socket| async move {
    ///         let mut session = broadcaster.connect_session(user_id).await;
    ///         // session deregisters automatically when this async block exits
    ///         while let Some(msg) = session.recv().await {
    ///             // forward message to WebSocket client
    ///         }
    ///     })
    /// }
    /// ```
    pub async fn connect_session(&self, user_id: Uuid) -> RealtimeSession {
        RealtimeSession::connect(self.registry.clone(), user_id).await
    }

    /// Publishes a domain event to Redis Pub/Sub for cross-replica distribution.
    ///
    /// Wraps `payload` in a [`RealtimeEvent`] envelope with a UUIDv7 `id`, UTC timestamp,
    /// and the specified `user_id` as target, then serializes to JSON and publishes to the
    /// configured Redis channel (`{channel_prefix}:events`).
    ///
    /// The subscriber loop on each replica will receive this event and forward it to all
    /// active connections registered for `user_id` in that replica's [`ConnectionRegistry`].
    ///
    /// # Arguments
    ///
    /// * `event_name` — Topic/name classifier for the event (e.g., `"chat.message"`, `"notification.created"`).
    /// * `user_id` — Target user UUID. Only connections registered for this user will receive the event.
    /// * `payload` — Any `Serialize`-able value to embed as the event payload.
    ///
    /// # Errors
    ///
    /// - [`RealtimeError::Serialization`] if `payload` cannot be serialized to JSON.
    /// - [`RealtimeError::Redis`] if the Redis PUBLISH command fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde::Serialize;
    /// use uuid::Uuid;
    ///
    /// #[derive(Serialize)]
    /// struct ChatMessage { text: String }
    ///
    /// broadcaster.publish("chat.message", user_id, &ChatMessage {
    ///     text: "Hello!".to_string(),
    /// }).await?;
    /// ```
    pub async fn publish<T: Serialize>(
        &self,
        event_name: &str,
        user_id: Uuid,
        payload: &T,
    ) -> Result<(), RealtimeError> {
        let event = RealtimeEvent::new(event_name, user_id, payload);
        self.publish_event(&event).await
    }

    /// Publishes a strongly-typed payload implementing [`RealtimePayload`] to a target user.
    ///
    /// Infers event topic name automatically from `<P as RealtimePayload>::EVENT_NAME`,
    /// avoiding magic string duplication at the call site.
    ///
    /// # Errors
    ///
    /// - [`RealtimeError::Serialization`] if `payload` cannot be serialized to JSON.
    /// - [`RealtimeError::Redis`] if the Redis PUBLISH command fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde::Serialize;
    /// use scyph_realtime::RealtimePayload;
    ///
    /// #[derive(Serialize)]
    /// struct UserUpdated { name: String }
    ///
    /// impl RealtimePayload for UserUpdated {
    ///     const EVENT_NAME: &'static str = "user.updated";
    /// }
    ///
    /// broadcaster.publish_payload(user_id, &UserUpdated { name: "Alice".to_string() }).await?;
    /// ```
    pub async fn publish_payload<P: RealtimePayload>(
        &self,
        user_id: Uuid,
        payload: &P,
    ) -> Result<(), RealtimeError> {
        self.publish(P::EVENT_NAME, user_id, payload).await
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, RealtimeError> {
        let mut guard = self.publisher.lock().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        let conn = self.client.get_multiplexed_async_connection().await?;
        *guard = Some(conn.clone());
        Ok(conn)
    }

    /// Publishes a domain event to multiple recipient users across all cluster replicas.
    ///
    /// Reuses a single multiplexed Redis connection across all recipient dispatches.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish_to_many<T: Serialize>(
        &self,
        event_name: &str,
        user_ids: &[Uuid],
        payload: &T,
    ) -> Result<(), RealtimeError> {
        let mut conn = self.get_connection().await?;
        let channel = format!("{}:events", self.config.channel_prefix);
        for &user_id in user_ids {
            let event = RealtimeEvent::new(event_name, user_id, payload);
            let raw = serde_json::to_string(&event)?;
            if let Err(e) = conn.publish::<_, _, ()>(&channel, raw).await {
                *self.publisher.lock().await = None;
                return Err(RealtimeError::from(e));
            }
        }
        Ok(())
    }

    /// Publishes a strongly-typed [`RealtimePayload`] to multiple recipient users across all cluster replicas.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish_payload_to_many<P: RealtimePayload>(
        &self,
        user_ids: &[Uuid],
        payload: &P,
    ) -> Result<(), RealtimeError> {
        self.publish_to_many(P::EVENT_NAME, user_ids, payload).await
    }

    /// Publishes a system-wide global event to ALL active WebSocket connections across ALL cluster replicas.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish_global<T: Serialize>(
        &self,
        event_name: &str,
        payload: &T,
    ) -> Result<(), RealtimeError> {
        let event = RealtimeEvent::new_global(event_name, payload);
        self.publish_event(&event).await
    }

    /// Publishes a strongly-typed [`RealtimePayload`] globally to ALL active WebSocket connections across ALL cluster replicas.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish_global_payload<P: RealtimePayload>(
        &self,
        payload: &P,
    ) -> Result<(), RealtimeError> {
        self.publish_global(P::EVENT_NAME, payload).await
    }

    /// Publishes a pre-constructed [`RealtimeEvent`] to Redis Pub/Sub.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish_event<T: Serialize>(
        &self,
        event: &RealtimeEvent<T>,
    ) -> Result<(), RealtimeError> {
        let payload = serde_json::to_string(event)?;
        let mut conn = self.get_connection().await?;
        let channel = format!("{}:events", self.config.channel_prefix);

        if let Err(e) = conn.publish::<_, _, ()>(channel, payload).await {
            *self.publisher.lock().await = None;
            return Err(RealtimeError::from(e));
        }
        Ok(())
    }

    /// Spawns the Redis subscriber task with graceful shutdown cancellation support.
    ///
    /// The subscriber loop:
    /// 1. Opens a dedicated Redis Pub/Sub connection (separate from the multiplexed publish connection)
    /// 2. Subscribes to `{channel_prefix}:events`
    /// 3. Deserializes incoming JSON messages into [`RawRealtimeEnvelope`] to extract routing metadata
    /// 4. Dispatches messages to local [`ConnectionRegistry`] channels using [`broadcast_local`](crate::registry::ConnectionRegistry::broadcast_local)
    ///    for user-targeted events or [`broadcast_global`](crate::registry::ConnectionRegistry::broadcast_global) for global broadcasts
    /// 5. On connection error, waits `reconnect_interval_secs` and retries automatically
    ///
    /// When `cancel_token` is cancelled (e.g., during `SIGTERM` or Axum graceful shutdown),
    /// the subscriber loop stops cleanly without waiting for the reconnect interval.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let cancel = CancellationToken::new();
    /// let handle = broadcaster.start_subscriber(cancel.clone());
    ///
    /// // Trigger shutdown:
    /// cancel.cancel();
    /// handle.await.unwrap();
    /// ```
    pub fn start_subscriber(&self, cancel_token: CancellationToken) -> tokio::task::JoinHandle<()> {
        let broadcaster = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Shutting down realtime subscriber task gracefully");
                }
                _ = broadcaster.run_subscriber_loop() => {}
            }
        })
    }

    /// Spawns the Redis subscriber task without explicit cancellation token.
    ///
    /// Creates an internal [`CancellationToken`] that is never cancelled, meaning the subscriber
    /// will run until the task is aborted or the process exits. Use [`start_subscriber`](Self::start_subscriber)
    /// if you need graceful shutdown support (recommended for production).
    pub fn spawn_subscriber(self) -> tokio::task::JoinHandle<()> {
        let cancel_token = CancellationToken::new();
        self.start_subscriber(cancel_token)
    }

    async fn run_subscriber_loop(&self) {
        let channel_name = format!("{}:events", self.config.channel_prefix);
        loop {
            if let Err(err) = self.subscribe_once(&channel_name).await {
                error!(error = %err, "Realtime subscriber error. Reconnecting in {}s...", self.config.reconnect_interval_secs);
                tokio::time::sleep(std::time::Duration::from_secs(
                    self.config.reconnect_interval_secs,
                ))
                .await;
            }
        }
    }

    async fn subscribe_once(&self, channel: &str) -> Result<(), RealtimeError> {
        let mut pubsub = self.client.get_async_pubsub().await?;

        pubsub.subscribe(channel).await?;
        info!(channel = %channel, "Realtime subscriber connected to Redis");

        let mut stream = pubsub.into_on_message();
        while let Some(msg) = stream.next().await {
            let raw: String = msg.get_payload().unwrap_or_default();
            if let Ok(env) = serde_json::from_str::<RawRealtimeEnvelope>(&raw) {
                if env.is_global {
                    self.registry.broadcast_global(&raw).await;
                } else if let Some(user_id) = env.user_id {
                    self.registry.broadcast_local(user_id, &raw).await;
                }
            }
        }
        Ok(())
    }
}
