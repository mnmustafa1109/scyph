//! Health check extension implementations for notification services.

use scyph_health::{HealthFailure, HealthRegistry};

#[cfg(feature = "email")]
use crate::email::LettreSMTPService;

#[cfg(feature = "fcm")]
use crate::fcm::FcmPushService;
#[cfg(feature = "fcm")]
use gcp_auth::TokenProvider;

#[cfg(feature = "email")]
impl LettreSMTPService {
    /// Checks SMTP server connection health and records the outcome in [`HealthRegistry`].
    /// Defaults `required` to `false` so SMTP issues do not cause `/readyz` probes to fail API traffic.
    pub async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "smtp_email", false).await;
    }

    /// Checks SMTP server connection health with a custom component identifier and readiness requirement.
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
    /// Checks FCM push notification service health and records the outcome in [`HealthRegistry`].
    /// Defaults `required` to `false` so FCM issues do not cause `/readyz` probes to fail API traffic.
    pub async fn check_health(&self, registry: &HealthRegistry) {
        self.check_health_named(registry, "fcm_push", false).await;
    }

    /// Checks FCM push notification service health with a custom component identifier and readiness requirement.
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
