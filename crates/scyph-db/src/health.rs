//! Extension trait providing one-line health checking for PostgreSQL connection pools.

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
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::{build_pool, DbHealthExt};
/// use scyph_health::HealthRegistry;
///
/// async fn monitor() {
///     let registry = HealthRegistry::new();
///     let pool = build_pool().await.expect("Pool");
///
///     // Run 1-line health check against default "database" identifier
///     pool.check_health(&registry).await;
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait DbHealthExt {
    /// Executes a `SELECT 1` ping query against the database pool and updates the health registry
    /// under default component name `"database"`.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared [`HealthRegistry`] instance to record database status.
    async fn check_health(&self, registry: &HealthRegistry);

    /// Executes a `SELECT 1` ping query against the database pool with a custom component identifier
    /// and readiness requirement.
    ///
    /// # Arguments
    ///
    /// * `registry` - Shared [`HealthRegistry`] instance.
    /// * `name` - Custom component name identifier (e.g. `"postgres_primary"`, `"read_replica"`).
    /// * `required` - If `true`, failure marks application readiness (`/readyz`) as `503`.
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
