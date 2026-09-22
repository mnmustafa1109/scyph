//! Firebase Cloud Messaging (FCM) push notification service.

use gcp_auth::{CustomServiceAccount, TokenProvider};
use reqwest::Client;
use scyph_core::AppError;
use serde_json::json;
use std::env;

use crate::{
    traits::{PushNotification, PushService},
    util::send_fcm_request,
};

/// FCM push notification service using Google Firebase v1 REST API.
pub struct FcmPushService {
    project_id: String,
    auth: CustomServiceAccount,
    client: Client,
}

impl FcmPushService {
    /// Constructs an [`FcmPushService`] from environment variables (`FCM_PROJECT_ID` and `FCM_SERVICE_ACCOUNT_JSON`).
    ///
    /// Performs an initial dry-run validation request against Firebase to verify permissions.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] if environment variables are missing, credentials are invalid,
    /// or dry-run validation fails.
    pub async fn from_env() -> Result<Self, AppError> {
        let project_id = env::var("FCM_PROJECT_ID")
            .map_err(|e| AppError::internal_from(e, "FCM_PROJECT_ID must be set"))?;

        let service_account_json = env::var("FCM_SERVICE_ACCOUNT_JSON")
            .map_err(|e| AppError::internal_from(e, "FCM_SERVICE_ACCOUNT_JSON must be set"))?;

        let auth = CustomServiceAccount::from_json(&service_account_json)
            .map_err(|e| AppError::internal_from(e, "Invalid FCM service account JSON"))?;

        let client = Client::new();

        // 1. Fetch access token to verify credentials and scope access
        let token = auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await
            .map_err(|e| AppError::internal_from(e, "Failed to acquire FCM OAuth token"))?;

        // 2. Dry-run request to test permissions (validate_only: true)
        let test_payload = json!({
            "validate_only": true,
            "message": {
                "topic": "healthcheck"
            }
        });

        send_fcm_request(&client, &project_id, token.as_str(), &test_payload).await?;

        Ok(Self {
            project_id,
            auth,
            client,
        })
    }
}

impl PushService for FcmPushService {
    async fn send(&self, n: PushNotification) -> Result<(), AppError> {
        let token = self
            .auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await
            .map_err(|e| AppError::internal_from(e, "FCM auth token"))?;

        let payload = json!({
            "message": {
                "token": n.token,
                "notification": {
                    "title": n.title,
                    "body": n.body
                },
                "data": n.data
            }
        });

        send_fcm_request(&self.client, &self.project_id, token.as_str(), &payload).await?;

        Ok(())
    }
}
