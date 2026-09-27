//! High-performance Base64 URL-safe UUID cursor pagination helpers.

use crate::error::UtilsError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque cursor wrapper around a 128-bit [`Uuid`] primary key or timestamp identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cursor {
    /// Target UUID identifier.
    pub id: Uuid,
}

impl Cursor {
    /// Encodes a [`Uuid`] into an opaque, URL-safe Base64 cursor string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_utils::pagination::Cursor;
    /// use uuid::Uuid;
    ///
    /// let id = Uuid::nil();
    /// let encoded = Cursor::encode(id);
    /// assert_eq!(Cursor::decode(&encoded).unwrap(), id);
    /// ```
    pub fn encode(id: Uuid) -> String {
        URL_SAFE_NO_PAD.encode(id.as_bytes())
    }

    /// Decodes an opaque URL-safe Base64 cursor string back into a [`Uuid`].
    ///
    /// # Errors
    ///
    /// Returns [`UtilsError::InvalidCursor`] if Base64 decoding fails or byte array is invalid.
    pub fn decode(s: &str) -> Result<Uuid, UtilsError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|e| UtilsError::InvalidCursor(e.to_string()))?;

        Uuid::from_slice(&bytes).map_err(|e| UtilsError::InvalidCursor(e.to_string()))
    }
}

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

impl PageParams {
    /// Returns the sanitized limit, clamped to a maximum of 100 records per page.
    pub fn limit(&self) -> i64 {
        self.limit.clamp(1, 100)
    }

    /// Decodes the optional `cursor` query string parameter into a [`Uuid`].
    ///
    /// # Errors
    ///
    /// Returns [`UtilsError::InvalidCursor`] if a cursor string is present but invalid.
    pub fn cursor_id(&self) -> Result<Option<Uuid>, UtilsError> {
        match &self.cursor {
            Some(s) if !s.trim().is_empty() => Cursor::decode(s).map(Some),
            _ => Ok(None),
        }
    }
}
