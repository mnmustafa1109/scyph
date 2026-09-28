//! Axum router endpoints for Kubernetes liveness (`/healthz`) and readiness (`/readyz`) probes.
//!
//! ## Endpoint Behavior
//!
//! ### `GET /healthz` — Liveness Probe
//!
//! Always returns `200 OK` with an empty body as long as the process is alive.
//! Kubernetes uses this to decide whether to restart a Pod.
//!
//! ### `GET /readyz` — Readiness Probe
//!
//! Queries the [`HealthRegistry`] and returns:
//!
//! - **`200 OK`** — all required components are [`Status::Healthy`].
//! - **`503 Service Unavailable`** — one or more required components are not healthy.
//!
//! Both success and failure responses include a JSON body with the full registry snapshot:
//!
//! ```json
//! {
//!   "database": {
//!     "status": "Healthy",
//!     "details": null,
//!     "required": true
//!   },
//!   "redis": {
//!     "status": "Sick",
//!     "details": "connection refused: 127.0.0.1:6379",
//!     "required": false
//!   }
//! }
//! ```
//!
//! The `Content-Type` is always `application/json`. The top-level keys are the component
//! names passed to [`HealthRegistry::check`] or [`HealthRegistry::set`]. Status values
//! are serialized as their variant names: `"Healthy"`, `"Sick"`, `"Recovering"`, `"Deceased"`.

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::registry::HealthRegistry;

/// Constructs an Axum [`Router`] configured with `/healthz` and `/readyz` health check endpoints.
///
/// - **`/healthz`**: Liveness probe returning `200 OK` with no body if the process is running.
///   This endpoint is stateless and has no dependency on the registry.
/// - **`/readyz`**: Readiness probe returning `200 OK` with a JSON registry snapshot if all
///   required services are healthy, or `503 Service Unavailable` with the same JSON snapshot
///   if any required service is unhealthy. The JSON body always reflects the full registry
///   state regardless of the HTTP status code.
///
/// The returned [`Router`] uses `registry` as its Axum state, making it compatible with
/// [`Router::merge`] and [`Router::nest`] when composing with a larger application router.
///
/// # Arguments
///
/// * `registry` - Shared [`HealthRegistry`] instance storing component statuses. Typically
///   the same registry that background health-check tasks write to.
///
/// # HTTP Response Format
///
/// See the [module-level documentation](self) for the full JSON response schema.
///
/// # Examples
///
/// ```rust
/// use scyph_health::{HealthRegistry, health_routes};
/// use axum::Router;
///
/// let registry = HealthRegistry::new();
///
/// // Mount at the root path
/// let app: Router = Router::new().merge(health_routes(registry.clone()));
///
/// // Or nest under a prefix
/// let app: Router = Router::new().nest("/health", health_routes(registry));
/// ```
pub fn health_routes(registry: HealthRegistry) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/readyz", get(readyz))
        .with_state(registry)
}

async fn readyz(State(registry): State<HealthRegistry>) -> impl IntoResponse {
    let snapshot = registry.snapshot().await;
    let ready = registry.is_ready().await;
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(snapshot))
}
