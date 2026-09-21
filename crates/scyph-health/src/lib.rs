//! # Scyph Health
//!
//! `scyph-health` provides thread-safe health status tracking ([`HealthRegistry`]) and
//! ready-to-use Axum routes for Kubernetes liveness (`/healthz`) and readiness (`/readyz`) probes.

#![warn(missing_docs)]

/// Service status tracking and health registry.
pub mod registry;

/// Axum router generators for `/healthz` and `/readyz` endpoints.
pub mod routes;

pub use registry::{HealthRegistry, ServiceStatus, Status};
pub use routes::health_routes;
