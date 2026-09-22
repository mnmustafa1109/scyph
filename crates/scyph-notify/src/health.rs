//! Automated health check extensions for notification services.
//!
//! Provides `check_health` and `check_health_named` extension methods for [`LettreSMTPService`] and [`FcmPushService`]
//! to register automated liveness and readiness monitoring checks with a [`HealthRegistry`].
//!
//! ### Non-Blocking Readiness Design
//!
//! By default, notification service health checks set `required = false`.
//! This ensures that transient network issues with external vendor endpoints (e.g. SMTP server timeout or Google FCM API degradation)
//! do not cause Kubernetes `/readyz` endpoints to mark API instances un-ready, keeping core HTTP request traffic flowing.

use scyph_health::{HealthFailure, HealthRegistry};

#[cfg(feature = "email")]
use crate::email::LettreSMTPService;

#[cfg(feature = "fcm")]
use crate::fcm::FcmPushService;
#[cfg(feature = "fcm")]
use gcp_auth::TokenProvider;

#[cfg(feature = "email")]
impl LettreSMTPService {
    /// Checks SMTP connection health and records the outcome in [`HealthRegistry`] under identifier `"smtp_email"`.
    ///
    /// Defaults `required` to `false` so SMTP connection issues do not block overall service readiness probes.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance to record the check outcome.
    pub async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "smtp_email", false).await;
    }

    /// Checks SMTP connection health with a custom component identifier name and readiness requirement.
    ///
    /// Executes an active SMTP test connection against the underlying transport pool.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    /// * `name` - Custom component name (e.g., `"primary_smtp"`).
    /// * `required` - If `true`, a health failure will mark the entire registry status as degraded/unhealthy.
    pub async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        let transport = self.transport().clone();
        registry
            .check(name, required, async move {
                match transport.test_connection().await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(HealthFailure::Transient(
                        "SMTP test connection returned negative response".to_string(),
                    )),
                    Err(e) => Err(HealthFailure::Transient(e.to_string())),
                }
            })
            .await;
    }
}

#[cfg(feature = "fcm")]
impl FcmPushService {
    /// Checks FCM push notification service health and records the outcome in [`HealthRegistry`] under identifier `"fcm_push"`.
    ///
    /// Defaults `required` to `false` so FCM API degradation does not block overall service readiness probes.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    pub async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "fcm_push", false).await;
    }

    /// Checks FCM push notification service health with a custom component identifier name and readiness requirement.
    ///
    /// Tests OAuth2 bearer token acquisition and performs a `"validate_only": true` dry-run request against Google FCM API.
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    /// * `name` - Custom component name (e.g., `"primary_fcm"`).
    /// * `required` - If `true`, a health failure will mark the entire registry status as degraded/unhealthy.
    pub async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        let token_res = self
            .auth()
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await;

        match token_res {
            Ok(token) => {
                let client = self.client().clone();
                let project_id = self.project_id().to_string();
                let test_payload = serde_json::json!({
                    "validate_only": true,
                    "message": { "topic": "healthcheck" }
                });

                registry
                    .check(name, required, async move {
                        crate::util::send_fcm_request(
                            &client,
                            &project_id,
                            token.as_str(),
                            &test_payload,
                        )
                        .await
                    })
                    .await;
            }
            Err(e) => {
                registry
                    .check(name, required, async move {
                        Err(HealthFailure::Fatal(format!(
                            "Failed to acquire FCM OAuth token: {e}"
                        )))
                    })
                    .await;
            }
        }
    }
}
