//! Health status registry tracking service liveness and readiness states.
//!
//! This module is the core of `scyph-health`. It defines:
//!
//! - [`HealthFailure`] — the two classes of health check failure.
//! - [`Status`] — the four possible operational states for a tracked component.
//! - [`ServiceStatus`] — a snapshot record storing a component's current state, details, and
//!   whether it is required for readiness.
//! - [`HealthRegistry`] — the thread-safe registry that aggregates all component statuses.
//! - [`IntoHealthResult`] — a blanket conversion trait that lets any `Result<T, E: Display>`
//!   be used as a health check future result without manually constructing [`HealthFailure`].
//!
//! ## Concurrency Model
//!
//! [`HealthRegistry`] wraps `Arc<RwLock<HashMap<String, ServiceStatus>>>`. Read operations
//! (`snapshot`, `is_ready`) acquire a read lock and clone the data, meaning multiple readers
//! can proceed concurrently. Write operations (`check`, `set`) acquire an exclusive write lock
//! only for the brief insert at the end of the check, not for the async work itself.
//!
//! The `unwrap_or_else(|e| e.into_inner())` pattern on lock acquisition ensures the registry
//! continues to function even if a previous lock holder panicked, by recovering the inner data
//! from the poison error.

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, RwLock},
};

use serde::Serialize;
use tracing::warn;

/// Classification of health check failure types.
///
/// Used to distinguish between failures that may resolve on their own (`Transient`) and
/// failures that require operator intervention (`Fatal`). The classification drives different
/// state machine transitions in [`HealthRegistry::check`]:
///
/// - [`HealthFailure::Transient`] → component moves to [`Status::Sick`] (can still recover).
/// - [`HealthFailure::Fatal`] → component moves to [`Status::Deceased`] (requires fix).
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_health::HealthFailure;
///
/// // Use Transient for temporary network errors
/// let err = HealthFailure::Transient("connection timeout".into());
///
/// // Use Fatal for misconfiguration or unrecoverable state
/// let err = HealthFailure::Fatal("invalid credentials".into());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HealthFailure {
    /// Transient error (e.g. timeout, temporary network issue) resulting in [`Status::Sick`].
    ///
    /// The component may recover on subsequent successful probes without operator action.
    Transient(String),
    /// Fatal error (e.g. invalid credentials, pool closed, unrecoverable state) resulting in [`Status::Deceased`].
    ///
    /// The component requires operator intervention to recover.
    Fatal(String),
}

/// Trait for converting health check outcomes into a categorized `Result<(), HealthFailure>`.
///
/// This blanket trait allows any `Result<T, E>` where `E: Display` to be passed directly
/// to [`HealthRegistry::check`] as the health check future result, without needing to
/// manually construct a [`HealthFailure`]. Non-`Ok` results are automatically classified
/// as [`HealthFailure::Transient`].
///
/// For fine-grained control over `Transient` vs `Fatal` classification, return
/// `Result<(), HealthFailure>` directly from your health check future — this type
/// also implements `IntoHealthResult` and passes through unchanged.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_health::{HealthRegistry, IntoHealthResult};
///
/// let registry = HealthRegistry::new();
///
/// // Any Result<T, E: Display> works — errors become Transient automatically
/// registry.check("db", true, async {
///     sqlx::query("SELECT 1").execute(&pool).await   // → Result<PgQueryResult, sqlx::Error>
/// }).await;
///
/// // Use Result<(), HealthFailure> for explicit Transient/Fatal distinction
/// registry.check("redis", true, async {
///     match redis_client.ping().await {
///         Ok(_) => Ok(()),
///         Err(e) if e.is_connection_closed() => Err(HealthFailure::Fatal(e.to_string())),
///         Err(e) => Err(HealthFailure::Transient(e.to_string())),
///     }
/// }).await;
/// ```
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
///
/// The four variants model a lifecycle from normal operation through degraded states to complete
/// failure, with a recovery path back to healthy. Only [`Status::Healthy`] components satisfy
/// the readiness check in [`HealthRegistry::is_ready`].
///
/// ## Serialization
///
/// Serializes to lowercase strings in JSON snapshots: `"Healthy"`, `"Sick"`, `"Recovering"`,
/// `"Deceased"` (using the default `#[derive(Serialize)]` behavior with serde's default casing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    /// Service is operating normally.
    ///
    /// Achieved when a component probe succeeds and the component was previously
    /// `Healthy` or entering from `Recovering`.
    Healthy,
    /// Service encountered errors or degraded performance.
    ///
    /// Entered when a probe returns a [`HealthFailure::Transient`] error.
    /// Can recover to `Recovering` → `Healthy` on subsequent successful probes.
    Sick,
    /// Service is in the process of self-healing or re-establishing connectivity.
    ///
    /// Intermediate state entered when a probe succeeds after the component was `Sick` or
    /// `Deceased`. Requires one additional successful probe to advance to `Healthy`.
    /// This two-probe recovery requirement prevents flapping on intermittent errors.
    Recovering,
    /// Service has failed completely and cannot recover without intervention.
    ///
    /// Entered when a probe returns a [`HealthFailure::Fatal`] error from any state.
    /// Can still enter `Recovering` if a subsequent probe succeeds, but this typically
    /// indicates the root cause was resolved externally (e.g. credentials rotated).
    Deceased,
}

/// Detailed health status record for a tracked service component.
///
/// Returned in the JSON snapshot from [`HealthRegistry::snapshot`] and serialized in the
/// `/readyz` response body. Each field is public so callers can inspect snapshots directly.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    /// Current operational status variant.
    pub status: Status,
    /// Optional failure details or error message context.
    ///
    /// Set to the error string when a probe fails. Set to a recovery note
    /// (`"Probe succeeded; component is re-stabilizing"`) when a previously failed component
    /// first succeeds again. `None` when the component is fully [`Status::Healthy`].
    pub details: Option<String>,
    /// Indicates whether this service must be [`Status::Healthy`] for readiness checks (`/readyz`) to pass.
    ///
    /// If `true` and the component's status is not `Healthy`, [`HealthRegistry::is_ready`]
    /// returns `false` and the `/readyz` endpoint responds with `503`.
    pub required: bool,
}

/// Thread-safe in-memory health status registry for monitoring application components.
///
/// Wraps an `Arc<RwLock<HashMap<String, ServiceStatus>>>` for concurrent read access across
/// async tasks. The `Arc` makes the registry cheap to clone and share — all clones point to
/// the same underlying data.
///
/// ## Usage Pattern
///
/// 1. Create a registry with `HealthRegistry::new()`.
/// 2. Clone it into any background tasks and the Axum application state.
/// 3. Background tasks call `registry.check(name, required, async { ... })` periodically.
/// 4. The `/readyz` route handler calls `registry.is_ready()` and `registry.snapshot()`.
///
/// ## Comprehensive Example
///
/// ```rust,ignore
/// use scyph_health::{HealthFailure, HealthRegistry, Status, health_routes};
/// use axum::Router;
/// use tokio::time::{Duration, interval};
///
/// #[tokio::main]
/// async fn main() {
///     let registry = HealthRegistry::new();
///
///     // ── Background: Database health check ────────────────────────────────
///     let db_registry = registry.clone();
///     tokio::spawn(async move {
///         let pool = build_db_pool().await;
///         let mut ticker = interval(Duration::from_secs(10));
///         loop {
///             ticker.tick().await;
///             // sqlx::Error maps to Transient automatically via IntoHealthResult
///             db_registry.check("database", true, async {
///                 sqlx::query("SELECT 1").execute(&pool).await
///             }).await;
///         }
///     });
///
///     // ── Background: Redis health check ────────────────────────────────────
///     let redis_registry = registry.clone();
///     tokio::spawn(async move {
///         let client = build_redis_client();
///         let mut ticker = interval(Duration::from_secs(15));
///         loop {
///             ticker.tick().await;
///             // Explicit Fatal vs Transient classification
///             redis_registry.check("redis", false, async {
///                 match client.ping().await {
///                     Ok(_) => Ok(()),
///                     Err(e) if e.is_auth_error() => Err(HealthFailure::Fatal(e.to_string())),
///                     Err(e) => Err(HealthFailure::Transient(e.to_string())),
///                 }
///             }).await;
///         }
///     });
///
///     // ── State machine transitions for "database" over time ────────────────
///     // Probe 1 (success)  → Status::Healthy
///     // Probe 2 (timeout)  → Status::Sick      (required=true → /readyz: 503)
///     // Probe 3 (success)  → Status::Recovering (still 503 — needs one more success)
///     // Probe 4 (success)  → Status::Healthy   (required=true → /readyz: 200)
///     // Probe N (fatal)    → Status::Deceased   (requires intervention)
///
///     // ── Axum: Attach /healthz and /readyz endpoints ───────────────────────
///     let app = Router::new().nest("/", health_routes(registry));
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
#[derive(Clone, Default)]
pub struct HealthRegistry(Arc<RwLock<HashMap<String, ServiceStatus>>>);

impl HealthRegistry {
    /// Constructs a new empty [`HealthRegistry`].
    pub fn new() -> Self {
        Self::default()
    }

    fn read_lock(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, ServiceStatus>> {
        self.0.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write_lock(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, ServiceStatus>> {
        self.0.write().unwrap_or_else(|e| e.into_inner())
    }

    /// Evaluates an async health check future, applying state machine transitions
    /// and updating the named component's entry in the registry.
    ///
    /// The async future `fut` is awaited first (outside any lock), then the result is
    /// classified and the registry is updated atomically under a write lock. This design
    /// ensures that slow health checks (e.g. a database query with a 5-second timeout) do
    /// not block readers of the registry.
    ///
    /// ## State Transitions
    ///
    /// | Previous State | Probe Result | Next State |
    /// |---------------|-------------|------------|
    /// | None (first)  | Success | `Healthy` |
    /// | `Healthy` | Success | `Healthy` |
    /// | `Sick` | Success | `Recovering` |
    /// | `Recovering` | Success | `Healthy` |
    /// | `Deceased` | Success | `Recovering` |
    /// | Any | `Transient` failure | `Sick` |
    /// | Any | `Fatal` failure | `Deceased` |
    ///
    /// When entering `Recovering`, `details` is set to `"Probe succeeded; component is re-stabilizing"`.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for the tracked service component (e.g. `"database"`, `"redis"`).
    ///   Must be unique within the registry — duplicate names overwrite the previous entry.
    /// * `required` - If `true`, failure marks the overall application as not ready
    ///   (`/readyz` returns `503 Service Unavailable`).
    /// * `fut` - Async health check future yielding any `Result<T, E>` where `E: Display`,
    ///   or `Result<(), HealthFailure>` for explicit Transient/Fatal classification.
    pub async fn check<F, R>(&self, name: &str, required: bool, fut: F)
    where
        F: Future<Output = R>,
        R: IntoHealthResult,
    {
        let prev_status = self.read_lock().get(name).map(|s| s.status);

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

                self.write_lock().insert(
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
                self.write_lock().insert(
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
                self.write_lock().insert(
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
    /// Bypasses the state machine transition logic of [`check`](Self::check) and directly
    /// writes the given `status` to the registry. Useful for:
    /// - Setting initial state before the first probe runs.
    /// - Manually marking a component as `Deceased` after an operator-detected failure.
    /// - Testing state machine behavior in unit tests.
    ///
    /// # Arguments
    ///
    /// * `name` - Service component identifier.
    /// * `status` - Operational [`Status`] variant to set.
    /// * `details` - Optional error message or status explanation string.
    /// * `required` - Whether this service is required for application readiness.
    pub async fn set(&self, name: &str, status: Status, details: Option<String>, required: bool) {
        self.write_lock().insert(
            name.to_string(),
            ServiceStatus {
                status,
                details,
                required,
            },
        );
    }

    /// Returns a point-in-time snapshot map of all tracked service statuses.
    ///
    /// Acquires a read lock, clones the entire registry map, and releases the lock. The
    /// returned `HashMap` is an independent copy — subsequent changes to the registry do not
    /// affect the returned snapshot.
    ///
    /// This is the data structure serialized into the `/readyz` JSON response body.
    pub async fn snapshot(&self) -> HashMap<String, ServiceStatus> {
        self.read_lock().clone()
    }

    /// Checks if all required service components are currently [`Status::Healthy`].
    ///
    /// Returns `true` only if every component where `required == true` has
    /// `status == Status::Healthy`. Components with `required == false` (optional services
    /// like read replicas or cache layers) do not affect the result.
    ///
    /// Returns `true` if the registry is empty (no components have been registered yet),
    /// which means the application starts in a ready state before any probes run. Use
    /// [`set`](Self::set) to pre-register components in a non-healthy state if you need the
    /// application to start in a not-ready condition.
    ///
    /// Returns `false` if any required component is `Sick`, `Recovering`, or `Deceased`.
    pub async fn is_ready(&self) -> bool {
        let snapshot = self.snapshot().await;
        snapshot
            .values()
            .filter(|s| s.required)
            .all(|s| s.status == Status::Healthy)
    }
}
