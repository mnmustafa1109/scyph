//! Pagination error types.
//!
//! Defines [`PaginationError`] — failures that occur when decoding opaque pagination cursor tokens.
//! These map to HTTP 400 Bad Request since they indicate the client sent a malformed cursor value.

use scyph_core::AppError;

/// Errors encountered during pagination cursor encoding or decoding.
///
/// ## When Does This Occur?
///
/// [`PaginationError::InvalidCursor`] surfaces when a client sends a `cursor` query parameter
/// that cannot be decoded as a valid Base64 URL-safe UUID. Common causes:
///
/// - Manually constructed or guessed cursor values.
/// - Cursor values from a different encoding scheme.
/// - Truncated or corrupted cursor strings in URL.
///
/// The error message includes the underlying Base64 or UUID parsing failure for debugging.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::pagination::{Cursor, PaginationError};
///
/// // Valid cursor round-trip
/// let id = uuid::Uuid::nil();
/// let encoded = Cursor::encode(id);
/// assert!(Cursor::decode(&encoded).is_ok());
///
/// // Invalid cursor → PaginationError::InvalidCursor
/// let result = Cursor::decode("not-valid-base64!!!");
/// assert!(matches!(result, Err(PaginationError::InvalidCursor(_))));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    /// Invalid or malformed cursor pagination token.
    #[error("Invalid pagination cursor token: {0}")]
    InvalidCursor(String),
}

impl From<PaginationError> for AppError {
    fn from(err: PaginationError) -> Self {
        match err {
            PaginationError::InvalidCursor(msg) => AppError::BadRequest(msg),
        }
    }
}
