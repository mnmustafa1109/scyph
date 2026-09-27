//! Automated health check extensions for storage services.
//!
//! Provides `check_health` and `check_health_named` extension methods for [`S3StorageService`]
//! to register automated liveness and readiness monitoring checks with a [`scyph_health::HealthRegistry`].

use scyph_health::HealthRegistry;

#[cfg(feature = "s3")]
use scyph_health::HealthFailure;

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
                match self.test_connection().await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("Forbidden")
                            || err_str.contains("AccessDenied")
                            || err_str.contains("InvalidAccessKeyId")
                            || err_str.contains("SignatureDoesNotMatch")
                            || err_str.contains("NoSuchBucket")
                        {
                            Err(HealthFailure::Fatal(err_str))
                        } else {
                            Err(HealthFailure::Transient(err_str))
                        }
                    }
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
