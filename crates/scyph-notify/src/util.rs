//! HTTP utility helpers for Firebase Cloud Messaging (FCM) API calls.

use reqwest::{Client, Response, StatusCode};
use scyph_core::AppError;
use serde_json::Value;

/// Sends an authenticated JSON HTTP POST request to the Google Firebase Cloud Messaging v1 API.
///
/// # Arguments
///
/// * `client` - Shared [`reqwest::Client`] instance.
/// * `project_id` - FCM project identifier.
/// * `access_token` - OAuth 2.0 access token string.
/// * `payload` - JSON payload value to send in the request body.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if the HTTP request fails or if the FCM endpoint returns an error status.
pub async fn send_fcm_request(
    client: &Client,
    project_id: &str,
    access_token: &str,
    payload: &Value,
) -> Result<Response, AppError> {
    let url = format!("https://fcm.googleapis.com/v1/projects/{project_id}/messages:send");

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(payload)
        .send()
        .await
        .map_err(|e| AppError::internal_from(e, "Failed to reach FCM endpoint"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == StatusCode::FORBIDDEN {
            return Err(AppError::internal(format!(
                "Service account does not have permission to send notifications for project '{project_id}': {body}"
            )));
        }

        return Err(AppError::internal(format!(
            "FCM request failed with status {status}: {body}"
        )));
    }

    Ok(response)
}
