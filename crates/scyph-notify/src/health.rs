//! Automated health check extensions for notification services.
//!
//! Provides `check_health` and `check_health_named` extension methods for [`LettreSMTPService`] and [`FcmPushService`]
//! to register automated liveness and readiness monitoring checks with a [`scyph_health::HealthRegistry`].
//!
//! ### Non-Blocking Readiness Design
//!
//! By default, notification service health checks set `required = false`.
//! This ensures that transient network issues with external vendor endpoints (e.g. SMTP server timeout or Google FCM API degradation)
//! do not cause Kubernetes `/readyz` endpoints to mark API instances un-ready, keeping core HTTP request traffic flowing.

// Gated like the impls below: with neither transport feature this module holds no code.
#[cfg(any(feature = "email", feature = "fcm"))]
use scyph_health::{HealthFailure, HealthRegistry};

#[cfg(any(feature = "email", feature = "fcm"))]
use std::time::Duration;

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
    /// The probe is bounded at 5s so a relay that accepts TCP and stalls mid-handshake
    /// (lettre's per-command timeout is 60s) cannot stall the caller's probe loop.
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
                match tokio::time::timeout(Duration::from_secs(5), transport.test_connection())
                    .await
                {
                    Err(_) => Err(HealthFailure::Transient(
                        "SMTP health probe timed out after 5s".to_string(),
                    )),
                    Ok(Ok(true)) => Ok(()),
                    Ok(Ok(false)) => Err(HealthFailure::Transient(
                        "SMTP test connection returned negative response".to_string(),
                    )),
                    Ok(Err(e)) => Err(HealthFailure::Transient(e.to_string())),
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

    // shortcut: the 5s bounds below still have no test — `gcp_auth` needs real
    // credentials and exposes no mock token provider, so the token fetch cannot be
    // pointed at a fake endpoint; the send path's base URL is now configurable via
    // `FcmPushService::base_url`, so only the OAuth token step remains untestable.
    // Upgrade when `FcmPushService` accepts an injectable token provider.
    /// Checks FCM push notification service health with a custom component identifier name and readiness requirement.
    ///
    /// Tests OAuth2 bearer token acquisition and performs a `"validate_only": true` dry-run request against Google FCM API.
    /// Both steps are bounded at 5s — neither the token fetch nor the FCM client sets a
    /// timeout — so a stalled endpoint costs one tick instead of freezing the caller's
    /// sequential probe loop (and `/readyz`'s view of every dependency after it).
    ///
    /// # Arguments
    ///
    /// * `registry` - Target [`HealthRegistry`] instance.
    /// * `name` - Custom component name (e.g., `"primary_fcm"`).
    /// * `required` - If `true`, a health failure will mark the entire registry status as degraded/unhealthy.
    pub async fn check_health_named(&self, registry: &HealthRegistry, name: &str, required: bool) {
        let token_res = match tokio::time::timeout(
            Duration::from_secs(5),
            self.auth()
                .token(&["https://www.googleapis.com/auth/firebase.messaging"]),
        )
        .await
        {
            Ok(Ok(token)) => Ok(token),
            Ok(Err(e)) => Err(HealthFailure::Fatal(format!(
                "Failed to acquire FCM OAuth token: {e}"
            ))),
            Err(_) => Err(HealthFailure::Transient(
                "FCM OAuth token request timed out after 5s".to_string(),
            )),
        };

        match token_res {
            Ok(token) => {
                let client = self.client().clone();
                let base_url = self.base_url().to_string();
                let project_id = self.project_id().to_string();
                let test_payload = serde_json::json!({
                    "validate_only": true,
                    "message": { "topic": "healthcheck" }
                });

                registry
                    .check(name, required, async move {
                        match tokio::time::timeout(
                            Duration::from_secs(5),
                            crate::util::send_fcm_request(
                                &client,
                                &base_url,
                                &project_id,
                                token.as_str(),
                                &test_payload,
                            ),
                        )
                        .await
                        {
                            Ok(Ok(_)) => Ok(()),
                            Ok(Err(e)) => Err(HealthFailure::Transient(e.to_string())),
                            Err(_) => Err(HealthFailure::Transient(
                                "FCM health probe timed out after 5s".to_string(),
                            )),
                        }
                    })
                    .await;
            }
            Err(failure) => {
                registry
                    .check(name, required, async move { Err(failure) })
                    .await;
            }
        }
    }
}
