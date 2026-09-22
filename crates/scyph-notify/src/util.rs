//! HTTP utility helpers for Firebase Cloud Messaging (FCM) API calls.
//!
//! Provides lower-level REST helpers for dispatching authenticated OAuth2 JSON payloads
//! to the Google Firebase Cloud Messaging v1 REST endpoint.

use crate::NotifyError;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;

/// Sends an authenticated JSON HTTP POST request to the Google Firebase Cloud Messaging v1 API (`https://fcm.googleapis.com/v1/projects/{project_id}/messages:send`).
///
/// Encapsulates Bearer token header formatting, JSON body serialization, and HTTP status code inspection.
///
/// # Arguments
///
/// * `client` - Reference to shared persistent [`reqwest::Client`].
/// * `project_id` - Google Cloud FCM project identifier string.
/// * `access_token` - OAuth 2.0 Bearer access token string acquired from `gcp_auth`.
/// * `payload` - JSON body payload value.
///
/// # Errors
///
/// Returns [`NotifyError::Configuration`] if HTTP 403 Forbidden is returned (indicating IAM scope or project permission issues),
/// or [`NotifyError::Internal`] / [`NotifyError::FcmHttp`] for transport and server errors.
pub async fn send_fcm_request(
    client: &Client,
    project_id: &str,
    access_token: &str,
    payload: &Value,
) -> Result<Response, NotifyError> {
    let url = format!("https://fcm.googleapis.com/v1/projects/{project_id}/messages:send");

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == StatusCode::FORBIDDEN {
            return Err(NotifyError::Configuration(format!(
                "Service account does not have permission to send notifications for project '{project_id}': {body}"
            )));
        }

        return Err(NotifyError::Internal(format!(
            "FCM request failed with status {status}: {body}"
        )));
    }

    Ok(response)
}
