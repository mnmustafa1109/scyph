//! Standard JSON envelope for single objects or lists.

use super::meta::ResponseMeta;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Standard JSON envelope for successful API responses.
///
/// Wraps response payload `data` alongside a `success` flag, a status `message`,
/// and optional response metadata [`ResponseMeta`].
/// When serialized, `data` and `meta` are omitted if `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// Indicates whether the API operation succeeded (always `true` for `ApiResponse`).
    pub success: bool,
    /// A human-readable status message (e.g. `"OK"`, `"Created"`, or custom message).
    pub message: String,
    /// Standardized request metadata (processing time, trace ID, timestamp, API version).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
    /// The response payload data, or `None` if no body data is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Optional HTTP status code when converting into an Axum response (defaults to 200 OK).
    #[serde(skip)]
    pub(crate) status: Option<StatusCode>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Creates a 200 OK [`ApiResponse`] envelope containing `data` with default message `"OK"`.
    ///
    /// # Arguments
    ///
    /// * `data` - Payload data to include in the response envelope.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ApiResponse;
    ///
    /// let res = ApiResponse::ok("User updated");
    /// assert_eq!(res.data, Some("User updated"));
    /// ```
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "OK".into(),
            meta: None,
            data: Some(data),
            status: Some(StatusCode::OK),
        }
    }

    /// Creates a 200 OK [`ApiResponse`] envelope containing `data` and a custom message.
    ///
    /// # Arguments
    ///
    /// * `data` - Payload data to include in the response envelope.
    /// * `msg` - Custom message string describing the outcome.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ApiResponse;
    ///
    /// let res = ApiResponse::ok_msg(42, "Calculated value");
    /// assert_eq!(res.message, "Calculated value");
    /// ```
    pub fn ok_msg(data: T, msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
            meta: None,
            data: Some(data),
            status: Some(StatusCode::OK),
        }
    }

    /// Creates a 201 Created [`ApiResponse`] envelope containing `data` with default message `"Created"`.
    ///
    /// # Arguments
    ///
    /// * `data` - Newly created entity data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::{ApiResponse, ResponseMeta};
    ///
    /// let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 5, "0.1.0");
    /// let res = ApiResponse::created("new_resource_id").with_meta(meta);
    /// assert_eq!(res.status(), axum::http::StatusCode::CREATED);
    /// ```
    pub fn created(data: T) -> Self {
        Self {
            success: true,
            message: "Created".into(),
            meta: None,
            data: Some(data),
            status: Some(StatusCode::CREATED),
        }
    }

    /// Sets a custom HTTP status code for this response envelope.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
    }

    /// Returns the HTTP status code configured for this response envelope (defaults to 200 OK).
    pub fn status(&self) -> StatusCode {
        self.status.unwrap_or(StatusCode::OK)
    }

    /// Attaches standardized request metadata [`ResponseMeta`] to the response envelope.
    ///
    /// # Arguments
    ///
    /// * `meta` - Request metadata containing processing time, trace ID, and version.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::{ApiResponse, ResponseMeta};
    ///
    /// let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 5, "0.1.0");
    /// let res = ApiResponse::ok("data").with_meta(meta);
    /// assert!(res.meta.is_some());
    /// ```
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl ApiResponse<()> {
    /// Creates a 204 No Content Axum response without a body.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ApiResponse;
    ///
    /// let response = ApiResponse::no_content();
    /// ```
    pub fn no_content() -> impl IntoResponse {
        StatusCode::NO_CONTENT
    }
}

impl<T: Serialize + Send> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = self.status.unwrap_or(StatusCode::OK);
        (status, Json(self)).into_response()
    }
}
