//! Storage error type definitions for `scyph-storage`.
//!
//! Categorizes all possible failures occurring during multipart parsing, MIME validation,
//! file size limits, AWS S3 operations, and environment configuration.

use scyph_core::{AppError, ErrorDetails};
use thiserror::Error;

/// Error type for all storage services and extractors in `scyph-storage`.
///
/// `StorageError` covers the full lifecycle of file upload and storage operations:
/// from multipart parsing and validation at the HTTP boundary, through to S3 transmission
/// and presigned URL generation.
///
/// # HTTP Status Mapping
///
/// `StorageError` implements `From<StorageError> for AppError` for automatic conversion in
/// Axum handlers. The mapping is:
///
/// | Variant | HTTP Status | Typical Cause |
/// |---|---|---|
/// | [`Configuration`](Self::Configuration) | 400 Bad Request | Missing `S3_BUCKET` or invalid env var |
/// | [`InvalidFile`](Self::InvalidFile) | 400 Bad Request | Missing field, wrong field name, no filename |
/// | [`UnsupportedMediaType`](Self::UnsupportedMediaType) | 422 Unprocessable Entity | MIME type not in allowlist or magic byte mismatch |
/// | [`FileTooLarge`](Self::FileTooLarge) | 422 Unprocessable Entity | File exceeds `FileConfig::max_size()` |
/// | [`TooManyFiles`](Self::TooManyFiles) | 422 Unprocessable Entity | Batch exceeds `FileConfig::max_files()` |
/// | [`ValidationError`](Self::ValidationError) | 422 Unprocessable Entity | Structured field-level validation failures |
/// | [`NotFound`](Self::NotFound) | 404 Not Found | Object key not in bucket or memory store |
/// | [`Presign`](Self::Presign) | 500 Internal Server Error | Signature calculation failure |
/// | [`S3`](Self::S3) | 500 Internal Server Error | S3 SDK transmission/auth error |
/// | [`Internal`](Self::Internal) | 500 Internal Server Error | Unclassified internal failure |
///
/// # Example
///
/// ```rust,ignore
/// use scyph_storage::{StorageService, StorageError};
/// use bytes::Bytes;
///
/// async fn upload_or_report(
///     storage: &impl StorageService,
///     path: &str,
/// ) {
///     match storage.retrieve(path).await {
///         Ok(data) => println!("Got {} bytes", data.len()),
///         Err(StorageError::NotFound(msg)) => eprintln!("Object not found: {msg}"),
///         Err(StorageError::S3(msg)) => eprintln!("S3 error: {msg}"),
///         Err(e) => eprintln!("Unexpected error: {e}"),
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum StorageError {
    /// Missing, empty, or unparseable configuration settings or environment variables.
    ///
    /// Examples: `S3_BUCKET` is not set, `S3_REGION` contains whitespace, or
    /// a required credential is missing from the environment.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Missing field, non-file field, or malformed multipart stream.
    ///
    /// Returned by [`FileExtractor`](crate::multipart::FileExtractor) when the expected
    /// multipart field is absent, lacks a filename, or the multipart boundary is invalid.
    #[error("Invalid file upload: {0}")]
    InvalidFile(String),

    /// Uploaded file MIME type is not in the allowed types list defined by [`FileConfig::allowed_mime_types`](crate::multipart::FileConfig::allowed_mime_types).
    ///
    /// Also returned when magic byte verification fails (binary content does not match
    /// the declared `Content-Type` header), preventing MIME-spoofing attacks.
    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),

    /// Uploaded file size exceeds the configured maximum byte threshold defined by [`FileConfig::max_size`](crate::multipart::FileConfig::max_size).
    ///
    /// Checked during streaming chunk accumulation to abort uploads early and prevent OOM.
    #[error("File too large: {0}")]
    FileTooLarge(String),

    /// Number of uploaded files in a batch exceeds the limit defined by [`FileConfig::max_files`](crate::multipart::FileConfig::max_files).
    ///
    /// Returned by [`MultiFileExtractor`](crate::multipart::MultiFileExtractor) when more
    /// files than the configured maximum are present in a single request.
    #[error("Too many files: {0}")]
    TooManyFiles(String),

    /// Structured validation failure for file upload parameters.
    ///
    /// Used when multiple field-level errors need to be reported together (e.g., in a form
    /// with several file upload fields). The `details` vector contains per-field error context.
    ///
    /// Construct via [`StorageError::validation`].
    #[error("Storage validation failed: {message}")]
    ValidationError {
        /// Summary validation error message.
        message: String,
        /// Field-level file upload error details.
        details: Vec<ErrorDetails>,
    },

    /// Object was not found in storage.
    ///
    /// Returned by [`StorageService::retrieve`](crate::traits::StorageService::retrieve) and
    /// the URL generation methods when the given path or key does not exist in the bucket or
    /// in-memory store.
    #[error("Object not found: {0}")]
    NotFound(String),

    /// Failure generating a cryptographically signed presigned URL.
    ///
    /// Occurs when the S3 SDK fails to compute a request signature, usually due to invalid
    /// credentials, clock skew, or malformed region configuration.
    #[error("Presign error: {0}")]
    Presign(String),

    /// AWS S3 SDK transmission, authentication, or network operation error.
    ///
    /// Covers connection timeouts, permission denied (403), bucket not found (404 at bucket
    /// level), throttling (503), and other S3-specific transport errors.
    #[error("S3 storage error: {0}")]
    S3(String),

    /// Generic internal storage processing error for unclassified failure paths.
    #[error("Storage error: {0}")]
    Internal(String),
}

impl StorageError {
    /// Constructs a [`StorageError::ValidationError`] with a summary message and field-level details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scyph_storage::StorageError;
    /// use scyph_core::ErrorDetails;
    ///
    /// let err = StorageError::validation(
    ///     "File upload validation failed",
    ///     vec![ErrorDetails {
    ///         code: "FILE_TOO_LARGE".into(),
    ///         message: "File too large".into(),
    ///         field: Some("avatar".into()),
    ///     }],
    /// );
    /// ```
    pub fn validation(message: impl Into<String>, details: Vec<ErrorDetails>) -> Self {
        Self::ValidationError {
            message: message.into(),
            details,
        }
    }
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
            StorageError::ValidationError { message, details } => {
                AppError::ValidationError { message, details }
            }
            StorageError::NotFound(msg) => AppError::NotFound(msg),
            StorageError::Presign(msg) => AppError::internal(format!("Presigning error: {msg}")),
            StorageError::S3(msg) => AppError::internal(format!("S3 storage error: {msg}")),
            StorageError::Internal(msg) => AppError::internal(msg),
        }
    }
}
