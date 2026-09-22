//! Firebase Cloud Messaging (FCM) push notification service.

use gcp_auth::{CustomServiceAccount, TokenProvider};
use reqwest::Client;
use scyph_core::AppError;
use secrecy::{ExposeSecret, SecretString};
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

        let service_account_json = SecretString::from(
            env::var("FCM_SERVICE_ACCOUNT_JSON")
                .map_err(|e| AppError::internal_from(e, "FCM_SERVICE_ACCOUNT_JSON must be set"))?,
        );

        let auth = CustomServiceAccount::from_json(service_account_json.expose_secret())
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

    /// Exposes a reference to the FCM project identifier string.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Exposes a reference to the inner [`CustomServiceAccount`].
    pub fn auth(&self) -> &CustomServiceAccount {
        &self.auth
    }

    /// Exposes a reference to the inner [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl PushService for FcmPushService {
    async fn send(&self, n: PushNotification) -> Result<(), AppError> {
        let token = self
            .auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await
            .map_err(|e| AppError::internal_from(e, "FCM auth token"))?;

        let mut notification_map = serde_json::Map::new();
        notification_map.insert("title".to_string(), json!(n.title));
        notification_map.insert("body".to_string(), json!(n.body));

        if let Some(image_url) = n.image {
            notification_map.insert("image".to_string(), json!(image_url));
        }

        let mut message_map = serde_json::Map::new();
        message_map.insert("token".to_string(), json!(n.token));
        message_map.insert("notification".to_string(), json!(notification_map));
        message_map.insert("data".to_string(), json!(n.data));

        if let Some(sound) = n.sound {
            message_map.insert(
                "android".to_string(),
                json!({
                    "notification": {
                        "sound": sound
                    }
                }),
            );
            message_map.insert(
                "apns".to_string(),
                json!({
                    "payload": {
                        "aps": {
                            "sound": sound
                        }
                    }
                }),
            );
        }

        let payload = json!({ "message": message_map });

        send_fcm_request(&self.client, &self.project_id, token.as_str(), &payload).await?;

        Ok(())
    }
}
