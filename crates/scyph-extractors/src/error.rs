//! Granular error types for request body, query, and path extraction and validation.

use scyph_core::AppError;

/// Granular errors encountered during payload extraction and validation.
#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    /// JSON request body deserialization failure.
    #[error("Invalid JSON body payload: {0}")]
    JsonParse(String),

    /// Query parameter parsing failure.
    #[error("Invalid query parameters: {0}")]
    QueryParse(String),

    /// Path parameter parsing failure.
    #[error("Invalid path parameter: {0}")]
    PathParse(String),

    /// `garde` validation rule violation.
    #[error("Validation failed: {0}")]
    Validation(String),
}

impl From<ExtractorError> for AppError {
    fn from(err: ExtractorError) -> Self {
        match err {
            ExtractorError::JsonParse(msg) => AppError::BadRequest(msg),
            ExtractorError::QueryParse(msg) => AppError::BadRequest(msg),
            ExtractorError::PathParse(msg) => AppError::BadRequest(msg),
            ExtractorError::Validation(msg) => AppError::UnprocessableEntity(msg),
        }
    }
}
