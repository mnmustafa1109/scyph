//! Standardized JSON response envelopes and pagination models.
//!
//! This module provides consistent API response structures across Scyph applications:
//! - [`ApiResponse<T>`]: General envelope for single objects, lists, or custom messages.
//! - [`PagedResponse<T>`]: Pagination envelope containing item lists and pagination metadata.
//!
//! # Examples
//!
//! ```rust
//! use scyph_core::ApiResponse;
//!
//! let response = ApiResponse::ok("Success data");
//! assert!(response.success);
//! assert_eq!(response.message, "OK");
//! ```

use crate::AppError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Standardized [`Result`](std::result::Result) type alias defaulting the error type to [`AppError`].                                                                                                      
pub type Result<T, E = AppError> = std::result::Result<T, E>;

/// Standard JSON envelope for successful API responses.
///
/// Wraps response payload `data` alongside a `success` flag and a status `message`.
/// When serialized, `data` is omitted if `None`.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Indicates whether the API operation succeeded (always `true` for `ApiResponse`).
    pub success: bool,
    /// A human-readable status message (e.g. `"OK"`, `"Created"`, or custom message).
    pub message: String,
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
                data: Some(data),
            }),
        )
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
#[derive(Debug, Serialize)]
pub struct PagedResponse<T: Serialize> {
    /// Indicates whether the API operation succeeded.
    pub success: bool,
    /// Status message string.
    pub message: String,
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
    /// Optional cursor token string for cursor-based pagination continuation.
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
    /// let paged = PagedResponse::new(vec!["item1", "item2"], 10, 1, 2);
    /// assert!(paged.has_next);
    /// assert_eq!(paged.total, 10);
    /// ```
    pub fn new(data: Vec<T>, total: i64, page: i64, per_page: i64) -> Self {
        let has_next = (page * per_page) < total;
        Self {
            success: true,
            message: "OK".into(),
            data,
            total,
            page,
            per_page,
            has_next,
            next_cursor: None,
        }
    }

    /// Attaches an optional cursor token string for cursor-based pagination continuation.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Continuation token string for fetching the next page.
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.next_cursor = Some(cursor.into());
        self
    }
}

impl<T: Serialize + Send> IntoResponse for PagedResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
