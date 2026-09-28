//! # Scyph Health
//!
//! `scyph-health` provides thread-safe health status tracking ([`HealthRegistry`]) and
//! ready-to-use Axum routes for Kubernetes liveness (`/healthz`) and readiness (`/readyz`) probes.
//!
//! ## Overview
//!
//! Modern Kubernetes-deployed applications require two health check endpoints:
//!
//! - **Liveness** (`/healthz`): Reports whether the process is running. If this fails,
//!   Kubernetes restarts the Pod. This endpoint always returns `200 OK` as long as the
//!   process itself is alive.
//! - **Readiness** (`/readyz`): Reports whether the application is ready to accept traffic.
//!   If this fails, Kubernetes removes the Pod from the load balancer. Returns `200 OK` when
//!   all *required* components are healthy, or `503 Service Unavailable` otherwise.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  HealthRegistry (Arc<RwLock<HashMap<name, ServiceStatus>>>)│
//! │                                                            │
//! │   "database"  → ServiceStatus { Healthy, required: true } │
//! │   "redis"     → ServiceStatus { Sick,    required: false } │
//! └──────────────────────────────────────────────────────────┘
//!         ↑                              ↓
//!   check() / set()            is_ready() / snapshot()
//!         ↑                              ↓
//!  [background tasks]          [/readyz route handler]
//! ```
//!
//! The [`HealthRegistry`] is an `Arc`-wrapped `RwLock` hash map. It is designed to be cloned
//! cheaply and shared across Axum application state and background health-check tasks.
//!
//! ## Status State Machine
//!
//! Each tracked component transitions through the following states:
//!
//! ```text
//! (new) ─── success ──► Healthy
//!                          │
//!                    transient err
//!                          │
//!                          ▼
//!                        Sick ◄── fatal err ── any state
//!                          │                       │
//!                       success               fatal err
//!                          │                       │
//!                          ▼                       ▼
//!                      Recovering             Deceased
//!                          │
//!                       success
//!                          │
//!                          ▼
//!                       Healthy
//! ```
//!
//! The `Recovering` intermediate state prevents flapping: a component that was `Sick` needs
//! **two** consecutive successful probes to return to `Healthy`.
//!
//! ## Quickstart
//!
//! ```rust,ignore
//! use axum::Router;
//! use scyph_health::{HealthRegistry, health_routes};
//! use tokio::time::{Duration, interval};
//!
//! #[tokio::main]
//! async fn main() {
//!     let registry = HealthRegistry::new();
//!
//!     // Spawn a background task that checks an external service every 15 seconds
//!     let r = registry.clone();
//!     tokio::spawn(async move {
//!         let mut ticker = interval(Duration::from_secs(15));
//!         loop {
//!             ticker.tick().await;
//!             r.check("redis", true, async {
//!                 my_redis_client.ping().await.map_err(|e| {
//!                     scyph_health::HealthFailure::Transient(e.to_string())
//!                 })
//!             }).await;
//!         }
//!     });
//!
//!     // Attach /healthz and /readyz routes to your Axum router
//!     let app = Router::new()
//!         .nest("/", health_routes(registry));
//!
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

#![warn(missing_docs)]

/// Service status tracking and health registry.
pub mod registry;

/// Axum router generators for `/healthz` and `/readyz` endpoints.
pub mod routes;

pub use registry::{HealthFailure, HealthRegistry, IntoHealthResult, ServiceStatus, Status};
pub use routes::health_routes;
