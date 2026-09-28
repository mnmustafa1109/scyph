//! Extension trait providing one-line health checking for PostgreSQL connection pools.
//!
//! This module provides [`DbHealthExt`], which adds `check_health` and `check_health_named`
//! methods directly on `sqlx::PgPool`. Both methods execute a `SELECT 1` ping query against
//! the pool and report the outcome to a [`HealthRegistry`].
//!
//! ## Error Classification
//!
//! Not all sqlx errors indicate the same severity. The private `classify_sqlx_error` function
//! maps `sqlx::Error` variants to [`HealthFailure`] levels:
//!
//! | `sqlx::Error` variant | → [`HealthFailure`] | Interpretation |
//! |----------------------|---------------------|----------------|
//! | `Configuration` | `Fatal` | Connection string is invalid — cannot recover without restart |
//! | `PoolClosed` | `Fatal` | Pool was explicitly closed — application is shutting down |
//! | `WorkerCrashed` | `Fatal` | Internal sqlx worker panicked — pool is unusable |
//! | All other errors | `Transient` | Timeout, network blip, or temporary overload — may self-heal |
//!
//! This drives the [`Status`](scyph_health::Status) state machine transitions in the registry:
//! transient failures move the component to `Sick` (recoverable), while fatal failures move
//! it to `Deceased` (requires intervention).
//!
//! ## Feature Gate
//!
//! This module is only compiled when the `health` feature flag is enabled:
//!
//! ```toml
//! [dependencies]
//! scyph-db = { version = "0.1", features = ["health"] }
//! ```

use scyph_health::{HealthFailure, HealthRegistry};
use sqlx::PgPool;

/// Classifies a [`sqlx::Error`] into a [`HealthFailure::Transient`] or [`HealthFailure::Fatal`] error.
fn classify_sqlx_error(err: &sqlx::Error) -> HealthFailure {
    match err {
        sqlx::Error::Configuration(msg) => {
            HealthFailure::Fatal(format!("Configuration error: {msg}"))
        }
        sqlx::Error::PoolClosed => {
            HealthFailure::Fatal("Database connection pool is closed".to_string())
        }
        sqlx::Error::WorkerCrashed => {
            HealthFailure::Fatal("Database worker thread crashed".to_string())
        }
        other => HealthFailure::Transient(other.to_string()),
    }
}

/// Extension trait enabling automated database health checks against a [`HealthRegistry`].
///
/// Implemented on `sqlx::PgPool`. Both methods execute a lightweight `SELECT 1` query
/// (not a full connection check) to verify that the pool can successfully dispatch a query
/// to PostgreSQL and receive a response within the pool's configured timeout.
///
/// ## Status Transitions
///
/// Each call updates the component's entry in the registry using the state machine in
/// [`HealthRegistry::check`]:
///
/// ```text
/// (first call, success)  → Healthy
/// Healthy  + success     → Healthy
/// Healthy  + transient   → Sick
/// Sick     + success     → Recovering
/// Recovering + success   → Healthy
/// Any state + fatal      → Deceased
/// Deceased + success     → Recovering
/// ```
///
/// A component in `Recovering` state requires **one additional successful probe** before it
/// returns to `Healthy`. This prevents flapping between healthy and unhealthy on intermittent
/// network issues.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::{build_pool, DbHealthExt};
/// use scyph_health::HealthRegistry;
/// use tokio::time::{Duration, interval};
///
/// #[tokio::main]
/// async fn main() {
///     let pool = build_pool().await.expect("Pool created");
///     let registry = HealthRegistry::new();
///
///     // One-shot health check using the default "database" component name
///     pool.check_health(&registry).await;
///
///     // Background health check loop running every 10 seconds
///     let pool_clone = pool.clone();
///     let registry_clone = registry.clone();
///     tokio::spawn(async move {
///         let mut ticker = interval(Duration::from_secs(10));
///         loop {
///             ticker.tick().await;
///             pool_clone.check_health(&registry_clone).await;
///         }
///     });
///
///     // Named check for a read replica — not required for readiness
///     pool.check_health_named(&registry, "read_replica", false).await;
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait DbHealthExt {
    /// Executes a `SELECT 1` ping query against the database pool and updates the health registry
    /// under the default component name `"database"`.
    ///
    /// Equivalent to calling `check_health_named(registry, "database", true)`. The component
    /// is marked as `required = true`, meaning a failure will cause `/readyz` to return `503`.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared [`HealthRegistry`] instance to record database status.
    async fn check_health(&self, registry: &HealthRegistry);

    /// Executes a `SELECT 1` ping query against the database pool with a custom component
    /// identifier and configurable readiness requirement.
    ///
    /// Use this variant when monitoring multiple pools (e.g. primary + read replica) or when
    /// a database component should not block Kubernetes readiness probes.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared [`HealthRegistry`] instance.
    /// * `name` - Custom component name identifier (e.g. `"postgres_primary"`, `"read_replica"`).
    ///   Must be unique within the registry — duplicate names overwrite the previous entry.
    /// * `required` - If `true`, failure marks application readiness (`/readyz`) as `503 Service Unavailable`.
    ///   If `false`, degraded status is recorded in the snapshot but does not block readiness.
    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool);
}

impl DbHealthExt for PgPool {
    async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "database", true).await;
    }

    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        registry
            .check(name, required, async {
                sqlx::query("SELECT 1")
                    .execute(self)
                    .await
                    .map(|_| ())
                    .map_err(|e| classify_sqlx_error(&e))
            })
            .await;
    }
}
