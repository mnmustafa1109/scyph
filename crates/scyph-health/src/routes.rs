//! Axum router endpoints for Kubernetes liveness (`/healthz`) and readiness (`/readyz`) probes.

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::registry::HealthRegistry;

/// Constructs an Axum [`Router`] configured with `/healthz` and `/readyz` health check endpoints.
///
/// - **`/healthz`**: Liveness probe returning `200 OK` if the process is running.
/// - **`/readyz`**: Readiness probe returning `200 OK` with JSON snapshot if all required services are healthy, or `503 Service Unavailable` if any required service is unhealthy.
///
/// # Arguments
///
/// * `registry` - Shared [`HealthRegistry`] instance storing component statuses.
///
/// # Examples
///
/// ```rust
/// use scyph_health::{HealthRegistry, health_routes};
/// use axum::Router;
///
/// let registry = HealthRegistry::new();
/// let app: Router = Router::new().nest("/api", health_routes(registry));
/// ```
pub fn health_routes(registry: HealthRegistry) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/readyz", get(readyz))
        .with_state(registry)
}

async fn readyz(State(registry): State<HealthRegistry>) -> impl IntoResponse {
    let snapshot = registry.snapshot();
    let ready = snapshot
        .values()
        .filter(|s| s.required)
        .all(|s| s.status == crate::registry::Status::Healthy);
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(snapshot))
}
