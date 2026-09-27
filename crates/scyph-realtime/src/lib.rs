#![warn(missing_docs)]
//! # Scyph Realtime
//!
//! `scyph-realtime` provides Redis Pub/Sub broadcasting for cross-replica WebSocket message fanout across Axum instances.
//!
//! ## Key Features & Architecture
//!
//! - **Cross-Replica Pub/Sub Fanout**: Publishes events to Redis Pub/Sub channels so all API replicas receive and route messages to target WebSocket connections.
//! - **Encapsulated Memory Management**: `ConnectionRegistry` tracks user connection handles and automatically purges empty entries to prevent memory leaks.
//! - **Structured Wire Protocol**: Standardized `RealtimeEvent<T>` envelopes with UUIDv7 timestamping and JSON payload delivery.
//! - **Graceful Task Cancellation**: Integrated `CancellationToken` support in `start_subscriber` for clean shutdown on SIGTERM.
//! - **Automated Health Probes ([`RealtimeHealthExt`])**: First-class integration with [`scyph_health::HealthRegistry`] for Kubernetes `/livez` and `/readyz` monitoring.

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

pub use broadcaster::RealtimeBroadcaster;
pub use config::RealtimeConfig;
pub use error::RealtimeError;
pub use event::{decode_event, RawRealtimeEnvelope, RealtimeEvent};
#[cfg(feature = "health")]
pub use health::RealtimeHealthExt;
pub use registry::{ConnectionRegistry, ConnectionTx, UserConnections};

