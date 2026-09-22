//! Extension trait providing one-line health checking for PostgreSQL connection pools.

use scyph_health::HealthRegistry;
use sqlx::PgPool;

/// Extension trait enabling automated database health checks against a [`HealthRegistry`].
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
                sqlx::query("SELECT 1").execute(self).await
            })
            .await;
    }
}
