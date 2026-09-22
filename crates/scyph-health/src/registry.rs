//! Health status registry tracking service liveness and readiness states.

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, RwLock},
};

use serde::Serialize;
use tracing::warn;

/// Classification of health check failure types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HealthFailure {
    /// Transient error (e.g. timeout, temporary network issue) resulting in [`Status::Sick`].
    Transient(String),
    /// Fatal error (e.g. invalid credentials, unrecoverable state) resulting in [`Status::Deceased`].
    Fatal(String),
}

/// Trait for converting health check outcomes into a categorized `Result<(), HealthFailure>`.
pub trait IntoHealthResult {
    /// Converts `self` into a [`Result<(), HealthFailure>`].
    fn into_health_result(self) -> Result<(), HealthFailure>;
}

impl<T, E: Display> IntoHealthResult for Result<T, E> {
    fn into_health_result(self) -> Result<(), HealthFailure> {
        self.map(|_| ())
            .map_err(|e| HealthFailure::Transient(e.to_string()))
    }
}

impl IntoHealthResult for Result<(), HealthFailure> {
    fn into_health_result(self) -> Result<(), HealthFailure> {
        self
    }
}

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

    /// Evaluates an async health check future, applying state machine transitions
    /// (`Sick` -> `Recovering` -> `Healthy`) and handling fatal errors (`Deceased`).
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for the tracked service component (e.g. `"database"`, `"redis"`).
    /// * `required` - If `true`, failure marks the overall application as not ready (`/readyz` returns `503`).
    /// * `fut` - Async health check future yielding any `Result<(), E>` or `Result<(), HealthFailure>`.
    pub async fn check<F, R>(&self, name: &str, required: bool, fut: F)
    where
        F: Future<Output = R>,
        R: IntoHealthResult,
    {
        let prev_status = self.0.read().unwrap().get(name).map(|s| s.status);

        match fut.await.into_health_result() {
            Ok(()) => {
                let next_status = match prev_status {
                    Some(Status::Sick) => Status::Recovering,
                    Some(Status::Recovering) | Some(Status::Healthy) | None => Status::Healthy,
                    Some(Status::Deceased) => Status::Recovering,
                };

                let details = if next_status == Status::Recovering {
                    Some("Probe succeeded; component is re-stabilizing".to_string())
                } else {
                    None
                };

                self.0.write().unwrap().insert(
                    name.to_string(),
                    ServiceStatus {
                        status: next_status,
                        details,
                        required,
                    },
                );
            }
            Err(HealthFailure::Transient(e)) => {
                warn!(service = name, error = %e, required, "Service health check failed (transient)");
                self.0.write().unwrap().insert(
                    name.to_string(),
                    ServiceStatus {
                        status: Status::Sick,
                        details: Some(e),
                        required,
                    },
                );
            }
            Err(HealthFailure::Fatal(e)) => {
                warn!(service = name, error = %e, required, "Service health check failed (fatal)");
                self.0.write().unwrap().insert(
                    name.to_string(),
                    ServiceStatus {
                        status: Status::Deceased,
                        details: Some(format!("Fatal failure: {e}")),
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
