//! Axum query parameter extractor for pagination requests.

use crate::pagination::{cursor::Cursor, error::PaginationError};
use serde::Deserialize;
use uuid::Uuid;

/// Query parameters extractor for Axum handlers (`Query<PageParams>`).
///
/// Automatically clamps `limit` between 1 and 100 (default: 20).
#[derive(Debug, Clone, Deserialize)]
pub struct PageParams {
    /// Maximum number of records to return (defaults to 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Optional opaque pagination cursor string.
    pub cursor: Option<String>,
}

fn default_limit() -> i64 {
    20
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            cursor: None,
        }
    }
}

impl PageParams {
    /// Creates a new `PageParams` instance with explicit limit and optional cursor.
    pub fn new(limit: i64, cursor: Option<impl Into<String>>) -> Self {
        Self {
            limit,
            cursor: cursor.map(Into::into),
        }
    }

    /// Returns the sanitized limit, clamped to a maximum of 100 records per page.
    pub fn limit(&self) -> i64 {
        self.limit.clamp(1, 100)
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
