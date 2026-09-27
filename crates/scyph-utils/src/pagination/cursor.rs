//! Base64 URL-safe UUID cursor encoding and decoding.

use crate::pagination::error::PaginationError;
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
    /// Returns [`PaginationError::InvalidCursor`] if Base64 decoding fails or byte array is invalid.
    pub fn decode(s: &str) -> Result<Uuid, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|e| PaginationError::InvalidCursor(e.to_string()))?;

        Uuid::from_slice(&bytes).map_err(|e| PaginationError::InvalidCursor(e.to_string()))
    }
}
