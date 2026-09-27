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
    event::{RawRealtimeEnvelope, RealtimeEvent},
    registry::{ConnectionRegistry, ConnectionTx},
};

/// High-performance Redis Pub/Sub broadcaster for cross-replica WebSocket message fanout.
#[derive(Debug, Clone)]
pub struct RealtimeBroadcaster {
    client: Client,
    config: RealtimeConfig,
    registry: ConnectionRegistry,
}

impl RealtimeBroadcaster {
    /// Constructs a [`RealtimeBroadcaster`] by reading environment variables using [`RealtimeConfig::from_env`].
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if environment loading or Redis client connection fails.
    pub fn from_env() -> Result<Self, RealtimeError> {
        let config = RealtimeConfig::from_env()?;
        Self::new(config)
    }

    /// Constructs a [`RealtimeBroadcaster`] with explicit [`RealtimeConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if opening Redis client fails.
    pub fn new(config: RealtimeConfig) -> Result<Self, RealtimeError> {
        let client = Client::open(config.redis_url.as_str())?;
        Ok(Self {
            client,
            config,
            registry: ConnectionRegistry::default(),
        })
    }

    /// Exposes a reference to the inner [`ConnectionRegistry`].
    pub fn registry(&self) -> &ConnectionRegistry {
        &self.registry
    }

    /// Exposes a reference to the active [`RealtimeConfig`].
    pub fn config(&self) -> &RealtimeConfig {
        &self.config
    }

    /// Exposes a reference to the underlying Redis [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Registers a new active WebSocket connection channel for a target user.
    pub async fn register(&self, user_id: Uuid, conn_id: Uuid, tx: ConnectionTx) {
        self.registry.register(user_id, conn_id, tx).await;
    }

    /// Deregisters an active WebSocket connection channel for a target user.
    pub async fn deregister(&self, user_id: Uuid, conn_id: Uuid) {
        self.registry.deregister(user_id, conn_id).await;
    }

    /// Publishes a domain event to Redis Pub/Sub for cross-replica distribution.
    ///
    /// Wraps `payload` in a [`RealtimeEvent`], serializes it to JSON, and publishes to the configured Redis channel.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError`] if serialization or Redis PUBLISH fails.
    pub async fn publish<T: Serialize>(
        &self,
        event_name: &str,
        user_id: Uuid,
        payload: &T,
    ) -> Result<(), RealtimeError> {
        let event = RealtimeEvent::new(event_name, user_id, payload);
        self.publish_event(&event).await
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
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let channel = format!("{}:events", self.config.channel_prefix);

        conn.publish::<_, _, ()>(channel, payload).await?;
        Ok(())
    }

    /// Spawns the Redis subscriber task with graceful shutdown cancellation support.
    ///
    /// When `cancel_token` is triggered (e.g. during SIGTERM / Axum shutdown), the subscriber loop stops cleanly.
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
    pub fn spawn_subscriber(self) -> tokio::task::JoinHandle<()> {
        let cancel_token = CancellationToken::new();
        self.start_subscriber(cancel_token)
    }

    async fn run_subscriber_loop(&self) {
        let channel_name = format!("{}:events", self.config.channel_prefix);
        loop {
            if let Err(err) = self.subscribe_once(&channel_name).await {
                error!(error = %err, "Realtime subscriber error. Reconnecting in {}s...", self.config.reconnect_interval_secs);
                tokio::time::sleep(std::time::Duration::from_secs(self.config.reconnect_interval_secs)).await;
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
                self.registry.broadcast_local(env.user_id, &raw).await;
            }
        }
        Ok(())
    }
}
