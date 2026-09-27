//! Automated health check extensions for realtime Redis broadcaster service.
//!
//! Provides `check_health` and `check_health_named` extension methods for [`RealtimeBroadcaster`]
//! to register automated liveness and readiness monitoring checks with a [`scyph_health::HealthRegistry`].

use redis::cmd;
use scyph_health::{HealthFailure, HealthRegistry};

use crate::broadcaster::RealtimeBroadcaster;

/// Extension trait enabling automated realtime broadcaster health checks against a [`HealthRegistry`].
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_health::HealthRegistry;
/// use scyph_realtime::{RealtimeBroadcaster, RealtimeHealthExt};
///
/// async fn monitor() {
///     let registry = HealthRegistry::new();
///     let broadcaster = RealtimeBroadcaster::from_env().expect("Broadcaster initialized");
///
///     // Run one-line health check against default "redis_realtime" component identifier
///     broadcaster.check_health(&registry).await;
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait RealtimeHealthExt {
    /// Executes a `PING` query against the Redis server and updates the health registry
    /// under default component identifier `"redis_realtime"`.
    ///
    /// Defaults `required` to `false` so transient Redis connectivity blips do not crash application readiness probes.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance to record probe outcome.
    async fn check_health(&self, registry: &HealthRegistry);

    /// Executes a `PING` query against the Redis server with a custom component identifier
    /// and readiness requirement.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    /// * `name` - Custom component name identifier (e.g. `"redis_primary"`, `"realtime_pubsub"`).
    /// * `required` - If `true`, probe failures will mark overall application readiness (`/readyz`) as `503`.
    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool);
}

/// Classifies a [`redis::RedisError`] into a [`HealthFailure::Transient`] or [`HealthFailure::Fatal`] error.
fn classify_redis_error(err: &redis::RedisError) -> HealthFailure {
    match err.kind() {
        redis::ErrorKind::AuthenticationFailed => {
            HealthFailure::Fatal(format!("Redis authentication failed: {err}"))
        }
        _ => {
            let msg = err.to_string();
            if msg.contains("WRONGPASS") || msg.contains("NOAUTH") {
                HealthFailure::Fatal(format!("Redis authentication failed: {msg}"))
            } else {
                HealthFailure::Transient(msg)
            }
        }
    }
}

impl RealtimeHealthExt for RealtimeBroadcaster {
    async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "redis_realtime", false).await;
    }

    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        let client = self.client().clone();
        registry
            .check(name, required, async move {
                let mut conn = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(|e| classify_redis_error(&e))?;

                let res: String = cmd("PING")
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| classify_redis_error(&e))?;

                if res.eq_ignore_ascii_case("PONG") {
                    Ok(())
                } else {
                    Err(HealthFailure::Transient(format!(
                        "Unexpected PING response from Redis: {res}"
                    )))
                }
            })
            .await;
    }
}
