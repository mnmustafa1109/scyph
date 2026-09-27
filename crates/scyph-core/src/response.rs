//! Standardized JSON response envelopes and pagination models.
//!
//! This module provides consistent API response structures across Scyph applications:
//! - [`ApiResponse<T>`]: General envelope for single objects, lists, or custom messages.
//! - [`PagedResponse<T>`]: Pagination envelope containing item lists and pagination metadata.
//! - [`ResponseMeta`]: Standardized metadata block containing latency, trace ID, timestamp, and version.
//!
//! # Examples
//!
//! ```rust
//! use scyph_core::{ApiResponse, ResponseMeta};
//!
//! let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 5, "0.1.0");
//! let response = ApiResponse::ok("Success data").with_meta(meta);
//! assert!(response.success);
//! assert_eq!(response.message, "OK");
//! assert!(response.meta.is_some());
//! ```

use crate::AppError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Standardized [`Result`](std::result::Result) type alias defaulting the error type to [`AppError`].
pub type Result<T, E = AppError> = std::result::Result<T, E>;

/// Standardized metadata header attached to API response envelopes.
///
/// Contains execution timing, request trace ID, timestamp, and application API version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Request processing latency in milliseconds.
    pub processing_time_ms: u64,
    /// ISO-8601 UTC timestamp of response generation.
    pub timestamp: String,
    /// Unique UUIDv7 request trace ID associated with the HTTP request.
    pub trace_id: String,
    /// API application version string.
    pub version: String,
}

impl ResponseMeta {
    /// Creates a new [`ResponseMeta`] instance with current UTC timestamp and given parameters.
    ///
    /// # Arguments
    ///
    /// * `trace_id` - Unique trace or request identifier.
    /// * `processing_time_ms` - Elapsed duration in milliseconds for handling the request.
    /// * `version` - API version string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ResponseMeta;
    ///
    /// let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 12, "0.1.0");
    /// assert_eq!(meta.processing_time_ms, 12);
    /// assert_eq!(meta.version, "0.1.0");
    /// assert_eq!(meta.trace_id, "01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9");
    /// ```
    pub fn new(
        trace_id: impl Into<String>,
        processing_time_ms: u64,
        version: impl Into<String>,
    ) -> Self {
        Self {
            processing_time_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.into(),
            version: version.into(),
        }
    }
}

/// Standard JSON envelope for successful API responses.
///
/// Wraps response payload `data` alongside a `success` flag, a status `message`,
/// and optional response metadata [`ResponseMeta`].
/// When serialized, `data` and `meta` are omitted if `None`.
#[derive(Debug, Serialize, Deserialize)]
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
        }
    }

    /// Creates a 201 Created Axum response containing [`ApiResponse`] payload.
    ///
    /// # Arguments
    ///
    /// * `data` - Newly created entity data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ApiResponse;
    /// use axum::response::IntoResponse;
    ///
    /// let res = ApiResponse::created("new_resource_id");
    /// ```
    pub fn created(data: T) -> impl IntoResponse {
        (
            StatusCode::CREATED,
            Json(Self {
                success: true,
                message: "Created".into(),
                meta: None,
                data: Some(data),
            }),
        )
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
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Standard envelope for paginated list responses.
///
/// Encapsulates page items along with offset/cursor pagination metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResponse<T: Serialize> {
    /// Indicates whether the API operation succeeded.
    pub success: bool,
    /// Status message string.
    pub message: String,
    /// Standardized request metadata (processing time, trace ID, timestamp, API version).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
    /// List of item records for the requested page.
    pub data: Vec<T>,
    /// Total count of records matching the query across all pages.
    pub total: i64,
    /// Current 1-indexed page number.
    pub page: i64,
    /// Number of records requested per page.
    pub per_page: i64,
    /// Indicates whether another page of records exists after this page.
    pub has_next: bool,
    /// Optional cursor token string for fetching the previous page in cursor-based pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Optional cursor token string for fetching the next page in cursor-based pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T: Serialize> PagedResponse<T> {
    /// Creates a new [`PagedResponse`] envelope with offset pagination metadata.
    ///
    /// # Arguments
    ///
    /// * `data` - Page item records vector.
    /// * `total` - Total count of matching records across all pages.
    /// * `page` - Current 1-indexed page number.
    /// * `per_page` - Number of items requested per page.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::PagedResponse;
    ///
    /// let paged = PagedResponse::new(vec!["item1", "item2"], 10, 1, 2)
    ///     .with_prev_cursor("cursor_prev_abc")
    ///     .with_next_cursor("cursor_next_xyz");
    /// assert!(paged.has_next);
    /// assert_eq!(paged.total, 10);
    /// assert_eq!(paged.prev_cursor.as_deref(), Some("cursor_prev_abc"));
    /// assert_eq!(paged.next_cursor.as_deref(), Some("cursor_next_xyz"));
    /// ```
    pub fn new(data: Vec<T>, total: i64, page: i64, per_page: i64) -> Self {
        let has_next = page.saturating_mul(per_page) < total;
        Self {
            success: true,
            message: "OK".into(),
            meta: None,
            data,
            total,
            page,
            per_page,
            has_next,
            prev_cursor: None,
            next_cursor: None,
        }
    }

    /// Attaches standardized request metadata [`ResponseMeta`] to the paginated response envelope.
    ///
    /// # Arguments
    ///
    /// * `meta` - Request metadata containing processing time, trace ID, and version.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::{PagedResponse, ResponseMeta};
    ///
    /// let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 8, "0.1.0");
    /// let paged = PagedResponse::new(vec!["item"], 1, 1, 10).with_meta(meta);
    /// assert!(paged.meta.is_some());
    /// ```
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Attaches an optional cursor token string for fetching the previous page in cursor pagination.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Continuation token string for fetching the previous page.
    pub fn with_prev_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.prev_cursor = Some(cursor.into());
        self
    }

    /// Attaches an optional cursor token string for fetching the next page in cursor pagination.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Continuation token string for fetching the next page.
    pub fn with_next_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.next_cursor = Some(cursor.into());
        self
    }

    /// Attaches an optional next page cursor token string for cursor-based pagination continuation.
    /// Alias for [`PagedResponse::with_next_cursor`].
    ///
    /// # Arguments
    ///
    /// * `cursor` - Continuation token string for fetching the next page.
    pub fn with_cursor(self, cursor: impl Into<String>) -> Self {
        self.with_next_cursor(cursor)
    }

    /// Attaches both optional previous and next cursor tokens for bi-directional cursor-based pagination.
    ///
    /// # Arguments
    ///
    /// * `prev` - Optional continuation token string for fetching the previous page.
    /// * `next` - Optional continuation token string for fetching the next page.
    pub fn with_cursors<S: Into<String>>(mut self, prev: Option<S>, next: Option<S>) -> Self {
        self.prev_cursor = prev.map(|s| s.into());
        self.next_cursor = next.map(|s| s.into());
        self
    }
}

impl<T: Serialize + Send> IntoResponse for PagedResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
