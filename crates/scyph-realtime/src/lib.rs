#![warn(missing_docs)]
//! # Scyph Realtime
//!
//! High-performance Redis Pub/Sub broadcasting for cross-replica WebSocket message fanout across Axum API nodes.
//!
//! ## Overview & Architecture
//!
//! In multi-replica or clustered deployments, WebSockets face a core challenge: a client connected to Replica 1 cannot receive events published by a client connected to Replica 2. `scyph-realtime` solves this using Redis Pub/Sub as a cross-replica messaging backplane:
//!
//! 1. **Cross-Replica Fanout ([`RealtimeBroadcaster`])**: Outgoing domain events are published to Redis Pub/Sub channels (`{channel_prefix}:events`). All API nodes subscribe to the channel and receive events.
//! 2. **Encapsulated Connection Registry ([`ConnectionRegistry`])**: Thread-safe in-memory connection registry tracks active WebSocket client handles (`ConnectionTx`) per user UUID. It automatically purges empty user connection entries when clients disconnect to prevent memory leaks.
//! 3. **Structured Wire Protocol ([`RealtimeEvent`])**: Standardized JSON envelopes timestamped with UUIDv7 identifiers for target user message routing and serialization.
//! 4. **Graceful Task Shutdown**: Integrated [`tokio_util::sync::CancellationToken`] support in `start_subscriber` for clean shutdown on `SIGTERM`.
//! 5. **Automated Health Probes ([`RealtimeHealthExt`])**: First-class integration with [`scyph_health::HealthRegistry`] for Kubernetes `/livez` and `/readyz` monitoring.
//!
//! ## Feature Flags
//!
//! - **`health`**: Enables [`RealtimeHealthExt`] extension trait for automated Redis liveness and readiness monitoring using [`scyph_health::HealthRegistry`].
//!
//! ## Quickstart Example (Without Health Feature)
//!
//! ```rust,ignore
//! use axum::{extract::{Path, State, WebSocketUpgrade, ws::WebSocket}, response::IntoResponse, routing::{get, post}, Json, Router};
//! use scyph_realtime::{RealtimeBroadcaster, RealtimeConfig};
//! use tokio::sync::mpsc;
//! use tokio_util::sync::CancellationToken;
//! use uuid::Uuid;
//!
//! #[derive(Clone)]
//! struct AppState {
//!     broadcaster: RealtimeBroadcaster,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RealtimeConfig::from_env()?;
//!     let broadcaster = RealtimeBroadcaster::new(config)?;
//!
//!     let cancel_token = CancellationToken::new();
//!     let _subscriber_handle = broadcaster.start_subscriber(cancel_token.clone());
//!
//!     let state = AppState { broadcaster };
//!     let app = Router::new()
//!         .route("/ws/{user_id}", get(ws_handler))
//!         .with_state(state);
//!
//!     Ok(())
//! }
//!
//! async fn ws_handler(
//!     ws: WebSocketUpgrade,
//!     Path(user_id): Path<Uuid>,
//!     State(state): State<AppState>,
//! ) -> impl IntoResponse {
//!     ws.on_upgrade(move |socket| handle_ws(socket, user_id, state.broadcaster))
//! }
//!
//! async fn handle_ws(mut socket: WebSocket, user_id: Uuid, broadcaster: RealtimeBroadcaster) {
//!     let conn_id = Uuid::now_v7();
//!     let (tx, mut rx) = mpsc::unbounded_channel();
//!     broadcaster.register(user_id, conn_id, tx).await;
//!
//!     while let Some(msg) = rx.recv().await {
//!         if socket.send(axum::extract::ws::Message::Text(msg.into())).await.is_err() {
//!             break;
//!         }
//!     }
//!     broadcaster.deregister(user_id, conn_id).await;
//! }
//! ```
//!
//! ## Quickstart Example (With Health Feature)
//!
//! ```rust,ignore
//! use scyph_health::HealthRegistry;
//! use scyph_realtime::{RealtimeBroadcaster, RealtimeHealthExt};
//! use std::time::Duration;
//!
//! async fn monitor_realtime(broadcaster: RealtimeBroadcaster, registry: HealthRegistry) {
//!     let mut interval = tokio::time::interval(Duration::from_secs(10));
//!     loop {
//!         interval.tick().await;
//!         // Probe Redis connectivity and update HealthRegistry status
//!         broadcaster.check_health_named(&registry, "redis_realtime", false).await;
//!     }
//! }
//! ```

/// Broadcaster client implementation and subscription loops.
pub mod broadcaster;

/// Configuration options and environment loader.
pub mod config;

/// Realtime error definitions.
pub mod error;

/// Realtime wire protocol event envelopes and helpers.
pub mod event;

#[cfg(feature = "health")]
/// Health check extensions for [`RealtimeBroadcaster`].
pub mod health;

/// In-memory connection registry data structures.
pub mod registry;

/// RAII WebSocket connection session handle.
pub mod session;

#[doc(inline)]
pub use broadcaster::RealtimeBroadcaster;
#[doc(inline)]
pub use config::RealtimeConfig;
#[doc(inline)]
pub use error::RealtimeError;
#[doc(inline)]
pub use event::{RawRealtimeEnvelope, RealtimeEvent, RealtimePayload, decode_event};
#[cfg(feature = "health")]
#[doc(inline)]
pub use health::RealtimeHealthExt;
#[doc(inline)]
pub use registry::{ConnectionRegistry, ConnectionTx, UserConnections};
#[doc(inline)]
pub use session::RealtimeSession;
