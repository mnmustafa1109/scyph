//! Firebase Cloud Messaging (FCM) push notification service powered by Google Firebase HTTP v1 REST API.
//!
//! ### HTTP Client Connection Pool Reuse
//!
//! `FcmPushService` maintains a persistent [`reqwest::Client`] instance.
//! All HTTP POST requests to FCM endpoints reuse this underlying HTTP/2 connection pool instead of re-establishing TCP/TLS handshakes per push event.

use gcp_auth::{CustomServiceAccount, TokenProvider};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use std::env;

use crate::{
    traits::{PushNotification, PushService},
    util::send_fcm_request,
    NotifyError,
};

/// FCM push notification service using Google Firebase v1 REST API.
///
/// Encapsulates Google Cloud Service Account credentials ([`CustomServiceAccount`]), FCM project ID,
/// and a persistent [`reqwest::Client`] connection pool.
pub struct FcmPushService {
    project_id: String,
    auth: CustomServiceAccount,
    client: Client,
}

impl FcmPushService {
    /// Constructs an [`FcmPushService`] instance from environment variables:
    /// - `FCM_PROJECT_ID` (e.g. `"my-firebase-project-id"`)
    /// - `FCM_SERVICE_ACCOUNT_JSON` (raw JSON contents of Google Service Account credentials file, sensitive).
    ///
    /// Performs an initial dry-run validation request (`"validate_only": true`) against Firebase to verify IAM permissions and API scope.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Configuration`] if environment variables are missing, service account JSON is invalid,
    /// or if the Firebase dry-run handshake fails.
    pub async fn from_env() -> Result<Self, NotifyError> {
        let project_id = env::var("FCM_PROJECT_ID")
            .map_err(|_| NotifyError::Configuration("FCM_PROJECT_ID must be set".into()))?;

        let service_account_json = SecretString::from(
            env::var("FCM_SERVICE_ACCOUNT_JSON")
                .map_err(|_| NotifyError::Configuration("FCM_SERVICE_ACCOUNT_JSON must be set".into()))?,
        );

        let auth = CustomServiceAccount::from_json(service_account_json.expose_secret())?;
        let client = Client::new();

        // 1. Fetch access token to verify credentials and scope access
        let token = auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await?;

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

    /// Exposes a reference to the FCM Google Cloud project identifier string.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Exposes a reference to the inner [`CustomServiceAccount`].
    pub fn auth(&self) -> &CustomServiceAccount {
        &self.auth
    }

    /// Exposes a reference to the inner persistent [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl PushService for FcmPushService {
    /// Sends a [`PushNotification`] payload asynchronously using Google Firebase Cloud Messaging v1 REST API.
    ///
    /// Fetches an OAuth2 bearer token from `gcp_auth`, formats FCM notification parameters,
    /// configures platform-specific sound settings for Android (`android.notification.sound`) and iOS APNS (`apns.payload.aps.sound`),
    /// and posts the JSON payload over the shared HTTP client pool.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if acquiring OAuth2 bearer tokens fails or if the FCM endpoint returns an HTTP error.
    async fn send(&self, n: PushNotification) -> Result<(), NotifyError> {
        let token = self
            .auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await?;

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
