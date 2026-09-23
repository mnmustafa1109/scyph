//! Storage error type definitions for `scyph-storage`.
//!
//! Categorizes all possible failures occurring during multipart parsing, MIME validation,
//! file size limits, AWS S3 operations, and environment configuration.

use scyph_core::error::AppError;
use thiserror::Error;

/// Error type for all storage services and extractors in `scyph-storage`.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Missing, empty, or unparseable configuration settings or environment variables (e.g. `S3_BUCKET`).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Missing field, non-file field, or malformed multipart stream.
    #[error("Invalid file upload: {0}")]
    InvalidFile(String),

    /// Uploaded file MIME type is not in the allowed types list.
    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),

    /// Uploaded file size exceeds the configured maximum byte threshold.
    #[error("File too large: {0}")]
    FileTooLarge(String),

    /// Number of uploaded files in a batch exceeds the configured limit.
    #[error("Too many files: {0}")]
    TooManyFiles(String),

    /// Object was not found in storage.
    #[error("Object not found: {0}")]
    NotFound(String),

    /// Failure generating cryptographically signed presigned URL.
    #[error("Presign error: {0}")]
    Presign(String),

    /// AWS S3 SDK transmission, authentication, or network operation error.
    #[error("S3 storage error: {0}")]
    S3(String),

    /// Generic internal storage processing error.
    #[error("Storage error: {0}")]
    Internal(String),
}

impl From<StorageError> for AppError {
    /// Maps a [`StorageError`] to the appropriate HTTP status code and RFC 7807 [`AppError`] response.
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Configuration(msg) => {
                AppError::BadRequest(format!("Storage configuration error: {msg}"))
            }
            StorageError::InvalidFile(msg) => AppError::BadRequest(msg),
            StorageError::UnsupportedMediaType(msg) => AppError::UnprocessableEntity(msg),
            StorageError::FileTooLarge(msg) => AppError::UnprocessableEntity(msg),
            StorageError::TooManyFiles(msg) => AppError::UnprocessableEntity(msg),
            StorageError::NotFound(msg) => AppError::NotFound(msg),
            StorageError::Presign(msg) => AppError::internal(format!("Presigning error: {msg}")),
            StorageError::S3(msg) => AppError::internal(format!("S3 storage error: {msg}")),
            StorageError::Internal(msg) => AppError::internal(msg),
        }
    }
}
