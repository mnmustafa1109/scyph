//! Standard JSON envelope for single objects or lists.
//!
//! This module provides [`ApiResponse<T>`], the canonical success response wrapper used
//! across all Scyph-based HTTP handlers. It ensures every successful response shares a
//! consistent JSON shape regardless of the underlying payload type.
//!
//! # Wire Format
//!
//! A typical `200 OK` response produced by [`ApiResponse::ok`] serializes as:
//!
//! ```json
//! {
//!   "success": true,
//!   "message": "OK",
//!   "data": {
//!     "id": "01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9",
//!     "name": "Alice",
//!     "email": "alice@example.com"
//!   }
//! }
//! ```
//!
//! When optional metadata is attached via [`ApiResponse::with_meta`], the `meta` key is included:
//!
//! ```json
//! {
//!   "success": true,
//!   "message": "OK",
//!   "meta": {
//!     "processing_time_ms": 5,
//!     "timestamp": "2026-09-28T12:00:00Z",
//!     "trace_id": "01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9",
//!     "version": "0.1.0"
//!   },
//!   "data": { "id": "...", "name": "Alice" }
//! }
//! ```
//!
//! When `data` is `None` (e.g. using `ApiResponse::no_content`), neither `data` nor `meta`
//! keys appear in the serialized JSON (controlled by `#[serde(skip_serializing_if = "Option::is_none")]`).
//!
//! # Design Notes
//!
//! - `ApiResponse<T>` implements [`IntoResponse`] directly,
//!   so handlers can return `Result<ApiResponse<T>, AppError>` uniformly across all HTTP status codes.
//! - All constructors (`ok`, `created`, `accepted`, `no_content`) return a concrete `ApiResponse<T>`,
//!   preserving consistent handler return types while correctly propagating the corresponding
//!   HTTP status code (200, 201, 202, 204) when converting to an Axum response.

use super::meta::ResponseMeta;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

fn default_status_code() -> StatusCode {
    StatusCode::OK
}

/// Standard JSON envelope for successful API responses.
///
/// Wraps response payload `data` alongside a `success` flag, a status `message`,
/// an HTTP status code, and optional response metadata [`ResponseMeta`].
/// When serialized, `data` and `meta` are omitted if `None`.
/// The `status` field is omitted from JSON serialization as it is communicated via the HTTP response status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// HTTP status code used when converted to an Axum response (default: `200 OK`).
    #[serde(skip, default = "default_status_code")]
    pub status: StatusCode,
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
            status: StatusCode::OK,
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
            status: StatusCode::OK,
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
    /// use scyph_core::ApiResponse;
    ///
    /// let res = ApiResponse::created("new_resource_id");
    /// assert_eq!(res.status, axum::http::StatusCode::CREATED);
    /// ```
    pub fn created(data: T) -> Self {
        Self {
            success: true,
            message: "Created".into(),
            meta: None,
            data: Some(data),
            status: StatusCode::CREATED,
        }
    }

    /// Creates a 201 Created [`ApiResponse`] envelope containing `data` and a custom message.
    ///
    /// # Arguments
    ///
    /// * `data` - Newly created entity data.
    /// * `msg` - Custom message string describing the creation outcome.
    pub fn created_msg(data: T, msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
            meta: None,
            data: Some(data),
            status: StatusCode::CREATED,
        }
    }

    /// Creates a 202 Accepted [`ApiResponse`] envelope containing `data` with default message `"Accepted"`.
    ///
    /// # Arguments
    ///
    /// * `data` - Payload data acknowledged for asynchronous processing.
    pub fn accepted(data: T) -> Self {
        Self {
            success: true,
            message: "Accepted".into(),
            meta: None,
            data: Some(data),
            status: StatusCode::ACCEPTED,
        }
    }

    /// Overrides the HTTP status code for this response.
    ///
    /// # Arguments
    ///
    /// * `status` - New HTTP status code to assign.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Returns the HTTP status code configured for this response.
    pub fn status(&self) -> StatusCode {
        self.status
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
    /// Creates a 204 No Content [`ApiResponse`] envelope without a body.
    ///
    /// When converted to an HTTP response via [`IntoResponse`], produces an HTTP 204 No Content
    /// status with an empty body.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ApiResponse;
    ///
    /// let response = ApiResponse::no_content();
    /// assert_eq!(response.status, axum::http::StatusCode::NO_CONTENT);
    /// ```
    pub fn no_content() -> Self {
        Self {
            success: true,
            message: "No Content".into(),
            meta: None,
            data: None,
            status: StatusCode::NO_CONTENT,
        }
    }
}

impl<T: Serialize + Send> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        if self.status == StatusCode::NO_CONTENT {
            StatusCode::NO_CONTENT.into_response()
        } else {
            (self.status, Json(self)).into_response()
        }
    }
}
