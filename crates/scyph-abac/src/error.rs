//! ABAC error type definitions for `scyph-abac`.

use scyph_core::AppError;

/// Error type for attribute-based access control policy evaluation in `scyph-abac`.
#[derive(Debug, thiserror::Error)]
pub enum AbacError {
    /// Authenticated subject does not satisfy policy conditions for the requested resource/action.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Missing or unauthenticated subject credentials.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Cedar policy engine evaluation or schema error.
    #[cfg(feature = "cedar")]
    #[error("Cedar policy error: {0}")]
    Cedar(#[from] crate::cedar::CedarError),

    /// Internal or generic ABAC policy evaluation failure.
    #[error("Internal ABAC error: {0}")]
    Internal(String),
}

impl From<AbacError> for AppError {
    /// Converts an [`AbacError`] into an RFC 7807 [`AppError`] response.
    fn from(err: AbacError) -> Self {
        match err {
            AbacError::Forbidden(msg) => AppError::Forbidden(msg),
            AbacError::Unauthorized(msg) => AppError::Unauthorized(msg),
            #[cfg(feature = "cedar")]
            AbacError::Cedar(e) => AppError::internal(e.to_string()),
            AbacError::Internal(msg) => AppError::internal(msg),
        }
    }
}
