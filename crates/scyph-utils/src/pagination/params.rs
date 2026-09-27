//! Axum query parameter extractor for pagination requests.

use crate::pagination::{cursor::Cursor, error::PaginationError};
use serde::Deserialize;
use uuid::Uuid;

/// Query parameters extractor for Axum handlers (`Query<PageParams>`).
///
/// Automatically clamps `limit` between [`PageParams::MIN_LIMIT`] and [`PageParams::MAX_LIMIT`] (default: [`PageParams::DEFAULT_LIMIT`]).
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
