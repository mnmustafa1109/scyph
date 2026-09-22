//! Authentication and authorization error type definitions for `scyph-auth`.

use scyph_core::AppError;

/// Error type for authentication, RBAC, and JWT operations in `scyph-auth`.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Missing, invalid, expired, or revoked authentication credentials (e.g. Bearer token).
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Authenticated user does not possess required role or permission.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Password hashing or verification failure.
    #[error("Password error: {0}")]
    Password(#[from] crate::password::PasswordError),

    /// JWT encoding, decoding, or verification failure.
    #[error("JWT error: {0}")]
    Jwt(#[from] crate::jwt::JwtError),

    /// Internal or generic authentication processing error.
    #[error("Internal auth error: {0}")]
    Internal(String),
}

impl From<AuthError> for AppError {
    /// Converts an [`AuthError`] into an RFC 7807 [`AppError`] response (401 Unauthorized, 403 Forbidden, or 500 Internal).
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Unauthorized(msg) => AppError::Unauthorized(msg),
            AuthError::Forbidden(msg) => AppError::Forbidden(msg),
            AuthError::Password(e) => AppError::Unauthorized(e.to_string()),
            AuthError::Jwt(e) => AppError::Unauthorized(e.to_string()),
            AuthError::Internal(msg) => AppError::internal(msg),
        }
    }
}
