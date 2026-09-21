//! Health status registry tracking service liveness and readiness states.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::Serialize;
use tracing::warn;

/// Operational status enumeration for a tracked service component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    /// Service is operating normally.
    Healthy,
    /// Service encountered errors or degraded performance.
    Sick,
    /// Service is in the process of self-healing or re-establishing connectivity.
    Recovering,
    /// Service has failed completely and cannot recover without intervention.
    Deceased,
}

/// Detailed health status record for a tracked service component.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    /// Current operational status variant.
    pub status: Status,
    /// Optional failure details or error message context.
    pub details: Option<String>,
    /// Indicates whether this service must be [`Status::Healthy`] for readiness checks (`/readyz`) to pass.
    pub required: bool,
}

/// Thread-safe in-memory health status registry for monitoring application components.
///
/// Wraps an `Arc<RwLock<HashMap<String, ServiceStatus>>>` for lock-free read access across tasks.
#[derive(Clone, Default)]
pub struct HealthRegistry(Arc<RwLock<HashMap<String, ServiceStatus>>>);

impl HealthRegistry {
    /// Constructs a new empty [`HealthRegistry`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates an async health check future and records the outcome in the registry.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for the tracked service component (e.g. `"database"`, `"redis"`).
    /// * `required` - If `true`, failure marks the overall application as not ready (`/readyz` returns `503`).
    /// * `fut` - Async health check future yielding `Result<(), E>`.
    pub async fn check<F, E>(&self, name: &str, required: bool, fut: F)
    where
        F: std::future::Future<Output = Result<(), E>>,
        E: std::fmt::Display,
    {
        match fut.await {
            Ok(()) => {
                self.0.write().unwrap().insert(
                    name.to_string(),
                    ServiceStatus {
                        status: Status::Healthy,
                        details: None,
                        required,
                    },
                );
            }
            Err(e) => {
                warn!(service = name, error = %e, required, "Service health check failed");
                self.0.write().unwrap().insert(
                    name.to_string(),
                    ServiceStatus {
                        status: Status::Sick,
                        details: Some(e.to_string()),
                        required,
                    },
                );
            }
        }
    }

    /// Manually sets the status of a tracked service component in the registry.
    ///
    /// # Arguments
    ///
    /// * `name` - Service component identifier.
    /// * `status` - Operational [`Status`] variant to set.
    /// * `details` - Optional error message or status explanation.
    /// * `required` - Whether this service is required for application readiness.
    pub async fn set(&self, name: &str, status: Status, details: Option<String>, required: bool) {
        self.0.write().unwrap().insert(
            name.to_string(),
            ServiceStatus {
                status,
                details,
                required,
            },
        );
    }

    /// Returns a point-in-time snapshot map of all tracked service statuses.
    pub async fn snapshot(&self) -> HashMap<String, ServiceStatus> {
        self.0.read().unwrap().clone()
    }

    /// Checks if all required service components are currently [`Status::Healthy`].
    ///
    /// Returns `true` if all required services are healthy, `false` otherwise.
    pub async fn is_ready(&self) -> bool {
        let snapshot = self.snapshot().await;
        snapshot
            .values()
            .filter(|s| s.required)
            .all(|s| s.status == Status::Healthy)
    }
}
