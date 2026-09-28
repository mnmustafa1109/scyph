//! Axum query parameter extractor for pagination requests.
//!
//! Provides [`PageParams`] — a serde-deserializable struct that captures the `cursor` and `limit`
//! query parameters from incoming requests and exposes them in a safe, clamped form for database queries.
//!
//! ## Cursor vs. Offset Pagination
//!
//! [`PageParams`] implements **cursor-based pagination** using opaque Base64-encoded UUID tokens.
//! Cursor pagination is preferred over offset pagination for large datasets because:
//!
//! - Consistent results: New rows inserted between pages do not cause items to be skipped or duplicated.
//! - Efficient queries: `WHERE id > cursor_id ORDER BY id` uses an index rather than scanning `OFFSET` rows.
//! - Stateless: The cursor encodes the exact position in the result set.
//!
//! ## Usage in Axum Handlers
//!
//! ```rust,ignore
//! use axum::extract::Query;
//! use scyph_utils::pagination::{Cursor, PageParams};
//! use uuid::Uuid;
//!
//! async fn list_products(Query(params): Query<PageParams>) -> axum::Json<serde_json::Value> {
//!     let limit = params.limit();                        // clamped [1, 100]
//!     let after_id: Option<Uuid> = params.cursor_id().unwrap_or(None);
//!
//!     // Query DB with cursor
//!     // SELECT * FROM products
//!     //   WHERE ($1::uuid IS NULL OR id > $1)
//!     //   ORDER BY id ASC
//!     //   LIMIT $2
//!     let mut rows = fetch_products(after_id, limit + 1).await;
//!     let page = Cursor::build_page(&mut rows, limit, |p| p.id);
//!     axum::Json(serde_json::to_value(page).unwrap())
//! }
//! ```

use crate::pagination::{cursor::Cursor, error::PaginationError};
use serde::Deserialize;
use uuid::Uuid;

/// Query parameters extractor for Axum handlers (`Query<PageParams>`).
///
/// Captures `limit` and `cursor` from the URL query string. Automatically clamps `limit` to
/// `[MIN_LIMIT, MAX_LIMIT]` when accessed via [`PageParams::limit`] (the raw field is preserved
/// for custom use cases).
///
/// ## Query String Format
///
/// ```text
/// GET /users?limit=50&cursor=AAAAAAAAAAAAAAAAAAAAAA
/// ```
///
/// - `limit` (optional, default: `20`): Number of items per page. Clamped to `[1, 100]`.
/// - `cursor` (optional): Opaque Base64 URL-safe string produced by [`Cursor::encode`] from the
///   previous page's last item ID. Omit for the first page.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::pagination::PageParams;
///
/// // Default params (first page, 20 items)
/// let params = PageParams::default();
/// assert_eq!(params.limit(), 20);
/// assert!(params.cursor_id().unwrap().is_none());
///
/// // Explicit page size
/// let params = PageParams::new(50, None::<String>);
/// assert_eq!(params.limit(), 50);
///
/// // Clamping: limit is clamped to MAX_LIMIT=100
/// let params = PageParams::new(9999, None::<String>);
/// assert_eq!(params.limit(), 100);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct PageParams {
    /// Maximum number of records to return (defaults to 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Optional opaque pagination cursor string.
    pub cursor: Option<String>,
}

fn default_limit() -> i64 {
    PageParams::DEFAULT_LIMIT
}

impl PageParams {
    /// Default number of items returned per page if unspecified.
    pub const DEFAULT_LIMIT: i64 = 20;
    /// Maximum allowable limit to prevent database query or memory exhaustion.
    pub const MAX_LIMIT: i64 = 100;
    /// Minimum allowable limit.
    pub const MIN_LIMIT: i64 = 1;

    /// Creates a new `PageParams` instance with explicit limit and optional cursor.
    pub fn new(limit: i64, cursor: Option<impl Into<String>>) -> Self {
        Self {
            limit,
            cursor: cursor.map(Into::into),
        }
    }

    /// Returns the sanitized limit, clamped between [`Self::MIN_LIMIT`] and [`Self::MAX_LIMIT`].
    pub fn limit(&self) -> i64 {
        self.limit.clamp(Self::MIN_LIMIT, Self::MAX_LIMIT)
    }

    /// Decodes the optional `cursor` query string parameter into a [`Uuid`].
    ///
    /// # Errors
    ///
    /// Returns [`PaginationError::InvalidCursor`] if a cursor string is present but invalid.
    pub fn cursor_id(&self) -> Result<Option<Uuid>, PaginationError> {
        match &self.cursor {
            Some(s) if !s.trim().is_empty() => Cursor::decode(s).map(Some),
            _ => Ok(None),
        }
    }
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            limit: Self::DEFAULT_LIMIT,
            cursor: None,
        }
    }
}
