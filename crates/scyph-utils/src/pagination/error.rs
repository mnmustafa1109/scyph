//! Pagination error types.

use scyph_core::AppError;

/// Errors encountered during pagination cursor encoding or decoding.
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
