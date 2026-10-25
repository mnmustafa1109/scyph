//! Automated health check extensions for storage services.
//!
//! Provides `check_health` and `check_health_named` extension methods for [`S3StorageService`]
//! to register automated liveness and readiness monitoring checks with a [`scyph_health::HealthRegistry`].

use scyph_health::HealthRegistry;

#[cfg(feature = "s3")]
use std::time::Duration;

#[cfg(feature = "s3")]
use scyph_health::HealthFailure;

#[cfg(feature = "s3")]
use crate::error::StorageError;
#[cfg(feature = "s3")]
use crate::s3::S3StorageService;
#[cfg(feature = "s3")]
use crate::traits::StorageService;

/// Extension trait enabling automated storage health checks against a [`HealthRegistry`].
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_health::HealthRegistry;
/// use scyph_storage::{S3StorageService, StorageHealthExt};
///
/// async fn monitor() {
///     let registry = HealthRegistry::new();
///     let storage = S3StorageService::from_env().await.expect("Storage initialized");
///
///     // Run one-line health check against default "s3_storage" component identifier
///     storage.check_health(&registry).await;
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait StorageHealthExt {
    /// Executes a connection test against the storage service and updates the health registry
    /// under default component identifier `"s3_storage"`.
    ///
    /// Failure severity is implementation-defined: the `S3StorageService` implementation
    /// classifies invalid credentials (`InvalidAccessKeyId` / `SignatureDoesNotMatch`) and a
    /// missing bucket (`NoSuchBucket`, HTTP 404) as fatal ([`Status::Deceased`](scyph_health::Status),
    /// operator action needed) and other failures as transient ([`Status::Sick`](scyph_health::Status)).
    /// An HTTP 403 without a credential error counts as reachable — logged warning,
    /// [`Status::Healthy`](scyph_health::Status) — matching `from_env()`'s boot decision to accept
    /// least-privilege credentials without `s3:ListBucket`: `is_ready()` rejects any non-`Healthy`
    /// required component (including `Sick` and `Recovering`), so anything else pins readiness at
    /// 503 for a deployment that boots and uploads fine. Each probe is bounded at 5s so a wedged
    /// endpoint cannot stall the caller's probe loop.
    ///
    /// Defaults `required` to `true` for application readiness (`/readyz`).
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance to record the probe outcome.
    async fn check_health(&self, registry: &HealthRegistry);

    /// Executes a connection test against the storage service with a custom component identifier
    /// and readiness requirement.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    /// * `name` - Custom component name identifier (e.g. `"primary_s3"`, `"backup_bucket"`).
    /// * `required` - If `true`, probe failures will mark overall application readiness (`/readyz`) as `503`.
    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool);
}

#[cfg(feature = "s3")]
impl StorageHealthExt for S3StorageService {
    async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "s3_storage", true).await;
    }

    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        registry
            .check(name, required, async move {
                // Bounded so an endpoint that accepts TCP and stops answering cannot
                // stall the caller's sequential probe loop forever.
                match tokio::time::timeout(Duration::from_secs(5), self.test_connection()).await {
                    Err(_) => Err(HealthFailure::Transient(
                        "S3 health probe timed out after 5s".to_string(),
                    )),
                    Ok(Ok(())) => Ok(()),
                    // A missing bucket needs an operator. A 403 is what from_env() already
                    // accepts at boot (least-privilege credentials without s3:ListBucket),
                    // and `is_ready()` rejects any non-Healthy required component — so
                    // anything else here pins /readyz at 503 for a deployment that boots
                    // and uploads fine. Warn like the boot check does, and count it ready.
                    Ok(Err(e @ StorageError::Forbidden(_))) => {
                        tracing::warn!(
                            error = %e,
                            "S3 health probe returned 403 Forbidden: the bucket is reachable \
                             but the credentials may lack the s3:ListBucket permission"
                        );
                        Ok(())
                    }
                    Ok(Err(e @ StorageError::NotFound(_))) => {
                        Err(HealthFailure::Fatal(e.to_string()))
                    }
                    // Wrong credentials came back named in the response body: no probe will
                    // fix them, so they are fatal, not something to wait out.
                    Ok(Err(e @ StorageError::Configuration(_))) => {
                        Err(HealthFailure::Fatal(e.to_string()))
                    }
                    Ok(Err(e)) => Err(HealthFailure::Transient(e.to_string())),
                }
            })
            .await;
    }
}

impl StorageHealthExt for crate::memory::InMemoryStorageService {
    async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "memory_storage", true)
            .await;
    }

    async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        registry
            .set(name, scyph_health::Status::Healthy, None, required)
            .await;
    }
}
