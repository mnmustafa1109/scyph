//! Standardized envelope for paginated responses.
//!
//! This module provides [`PagedResponse<T>`], the canonical wrapper for list endpoints that
//! return paginated data. It supports both **offset-based** and **cursor-based** pagination
//! in a single struct, omitting cursor fields when they are not used.
//!
//! # Wire Format
//!
//! A typical `200 OK` offset-paginated response from [`PagedResponse::new`] serializes as:
//!
//! ```json
//! {
//!   "success": true,
//!   "message": "OK",
//!   "data": [
//!     { "id": "...", "name": "Alice" },
//!     { "id": "...", "name": "Bob" }
//!   ],
//!   "total": 42,
//!   "page": 1,
//!   "per_page": 20,
//!   "has_next": true
//! }
//! ```
//!
//! When cursor-based pagination tokens are attached via [`PagedResponse::with_cursors`]:
//!
//! ```json
//! {
//!   "success": true,
//!   "message": "OK",
//!   "data": [...],
//!   "total": 42,
//!   "page": 2,
//!   "per_page": 20,
//!   "has_next": true,
//!   "prev_cursor": "eyJpZCI6IjAxOTIzZiJ9",
//!   "next_cursor": "eyJpZCI6IjAxOTI0MCJ9"
//! }
//! ```
//!
//! # Design Notes
//!
//! - `has_next` is computed automatically by [`PagedResponse::new`] as `page * per_page < total`.
//! - Cursor fields (`prev_cursor`, `next_cursor`) are omitted from JSON when `None`
//!   via `#[serde(skip_serializing_if = "Option::is_none")]`.
//! - You can use offset pagination, cursor pagination, or both simultaneously in the same response.
//! - [`PagedResponse`] implements [`IntoResponse`], so it can be
//!   returned directly from Axum handlers.

use super::meta::ResponseMeta;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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
