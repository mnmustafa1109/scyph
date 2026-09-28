//! ABAC error type definitions for `scyph-abac`.
//!
//! Defines [`AbacError`] — the unified error type for all attribute-based access control
//! policy enforcement failures in `scyph-abac`.
//!
//! ## Error Hierarchy and HTTP Mapping
//!
//! | Variant | HTTP Status (via `AppError`) | Cause |
//! |---------|------------------------------|-------|
//! | `Forbidden` | 403 Forbidden | User attributes do not satisfy policy conditions |
//! | `Unauthorized` | 401 Unauthorized | No authenticated principal provided |
//! | `ValidationError` | 422 Unprocessable Entity | Policy attribute validation failed |
//! | `Cedar` | 500 Internal Server Error | Cedar engine evaluation/parse error |
//! | `Internal` | 500 Internal Server Error | Generic ABAC infrastructure failure |
//!
//! ## Conversion to `AppError`
//!
//! `AbacError` implements `From<AbacError> for AppError`, enabling seamless propagation
//! with `?` in Axum handlers that return `Result<_, AppError>`:
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AbacError, AuthUserEnforceExt};
//! use scyph_core::{Action, AppError, ApiResponse};
//!
//! async fn get_document(user: AuthUser<AppClaims>, doc: Document) -> Result<ApiResponse<Document>, AppError> {
//!     user.enforce::<DocumentPolicy>(&doc, Action::Read)?;  // AbacError -> AppError via ?
//!     Ok(ApiResponse::ok(doc))
//! }
//! ```

use scyph_core::{AppError, ErrorDetails};

/// Error type for attribute-based access control policy evaluation in `scyph-abac`.
#[derive(Debug, thiserror::Error)]
pub enum AbacError {
    /// Authenticated subject does not satisfy policy conditions for the requested resource/action.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Missing or unauthenticated subject credentials.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Structured validation failure for policy attributes or context inputs.
    #[error("ABAC validation failed: {message}")]
    ValidationError {
        /// Summary validation failure message.
        message: String,
        /// Field-level policy attribute error details.
        details: Vec<ErrorDetails>,
    },

    /// Cedar policy engine evaluation or schema error.
    #[cfg(feature = "cedar")]
    #[error("Cedar policy error: {0}")]
    Cedar(#[from] crate::cedar::CedarError),

    /// Internal or generic ABAC policy evaluation failure.
    #[error("Internal ABAC error: {0}")]
    Internal(String),
}

impl AbacError {
    /// Constructs an [`AbacError::ValidationError`] with a summary message and field-level details.
    pub fn validation(message: impl Into<String>, details: Vec<ErrorDetails>) -> Self {
        Self::ValidationError {
            message: message.into(),
            details,
        }
    }
}

impl From<AbacError> for AppError {
    /// Converts an [`AbacError`] into an RFC 7807 [`AppError`] response.
    fn from(err: AbacError) -> Self {
        match err {
            AbacError::Forbidden(msg) => AppError::Forbidden(msg),
            AbacError::Unauthorized(msg) => AppError::Unauthorized(msg),
            AbacError::ValidationError { message, details } => {
                AppError::ValidationError { message, details }
            }
            #[cfg(feature = "cedar")]
            AbacError::Cedar(e) => AppError::internal(e.to_string()),
            AbacError::Internal(msg) => AppError::internal(msg),
        }
    }
}
