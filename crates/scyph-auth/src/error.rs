//! Authentication and authorization error type definitions for `scyph-auth`.
//!
//! This module provides [`AuthError`], the unified error type for all authentication,
//! RBAC, JWT, and password operations within `scyph-auth`. It implements `Into<AppError>`
//! to integrate seamlessly with Scyph's RFC 7807 HTTP error response system.
//!
//! # Error Mapping
//!
//! | [`AuthError`] variant  | [`scyph_core::AppError`] / HTTP status |
//! |------------------------|-----------------------------------|
//! | `Unauthorized`         | `AppError::Unauthorized` / 401    |
//! | `Forbidden`            | `AppError::Forbidden` / 403       |
//! | `ValidationError`      | `AppError::ValidationError` / 422 |
//! | `Password`             | `AppError::Unauthorized` / 401    |
//! | `Jwt`                  | `AppError::Unauthorized` / 401    |
//! | `Internal`             | `AppError::internal(...)` / 500   |
//!
//! # Examples
//!
//! Constructing a validation error with field details:
//!
//! ```rust
//! use scyph_auth::AuthError;
//! use scyph_core::ErrorDetails;
//!
//! let err = AuthError::validation(
//!     "Password policy violation",
//!     vec![
//!         ErrorDetails {
//!             code: "TOO_SHORT".into(),
//!             message: "Password must be at least 12 characters".into(),
//!             field: Some("password".into()),
//!         },
//!     ],
//! );
//!
//! // Convert to AppError for HTTP response
//! let app_error: scyph_core::AppError = err.into();
//! assert_eq!(app_error.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
//! ```
//!
//! Returning an `AuthError` from a handler (auto-converts via `Into<AppError>`):
//!
//! ```rust,ignore
//! use scyph_auth::AuthError;
//! use scyph_core::{AppError, ApiResponse};
//!
//! async fn login_handler(/* ... */) -> Result<ApiResponse<String>, AppError> {
//!     // verify_password returns PasswordError which converts to AuthError then AppError
//!     scyph_auth::verify_password_async(&input_password, &stored_hash).await
//!         .map_err(|e| AppError::from(AuthError::from(e)))?;
//!     Ok(ApiResponse::ok("token_string".into()))
//! }
//! ```

use scyph_core::{AppError, ErrorDetails};

/// Error type for authentication, RBAC, and JWT operations in `scyph-auth`.
///
/// All variants convert to [`AppError`] via the [`From<AuthError> for AppError`] implementation,
/// making it straightforward to use `?` in handlers that return `Result<_, AppError>`.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Missing, invalid, expired, or revoked authentication credentials (e.g. Bearer token).
    ///
    /// Maps to HTTP 401 Unauthorized.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Authenticated user does not possess required role or permission.
    ///
    /// Maps to HTTP 403 Forbidden.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Structured validation failure (e.g. password policy, invalid credential fields).
    ///
    /// Maps to HTTP 422 Unprocessable Entity with per-field [`ErrorDetails`].
    #[error("Auth validation failed: {message}")]
    ValidationError {
        /// Summary message.
        message: String,
        /// Field-level validation error details.
        details: Vec<ErrorDetails>,
    },

    /// Password hashing or verification failure.
    ///
    /// Wraps [`PasswordError`](crate::password::PasswordError). Maps to HTTP 401 Unauthorized
    /// because a password failure indicates invalid credentials.
    #[error("Password error: {0}")]
    Password(#[from] crate::password::PasswordError),

    /// JWT encoding, decoding, or verification failure.
    ///
    /// Wraps [`JwtError`](crate::jwt::JwtError). Maps to HTTP 401 Unauthorized.
    #[error("JWT error: {0}")]
    Jwt(#[from] crate::jwt::JwtError),

    /// Internal or generic authentication processing error.
    ///
    /// Maps to HTTP 500 Internal Server Error.
    #[error("Internal auth error: {0}")]
    Internal(String),
}

impl AuthError {
    /// Constructs an [`AuthError::ValidationError`] with a summary message and field-level details.
    ///
    /// # Arguments
    ///
    /// * `message` - A human-readable summary of why validation failed.
    /// * `details` - A list of [`ErrorDetails`] describing per-field failures.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_auth::AuthError;
    /// use scyph_core::ErrorDetails;
    ///
    /// let err = AuthError::validation(
    ///     "Invalid credentials",
    ///     vec![ErrorDetails {
    ///         code: "REQUIRED".into(),
    ///         message: "Email is required".into(),
    ///         field: Some("email".into()),
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

impl From<AuthError> for AppError {
    /// Converts an [`AuthError`] into an RFC 7807 [`AppError`] response (401 Unauthorized, 403 Forbidden, 422 Unprocessable, or 500 Internal).
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Unauthorized(msg) => AppError::Unauthorized(msg),
            AuthError::Forbidden(msg) => AppError::Forbidden(msg),
            AuthError::ValidationError { message, details } => {
                AppError::ValidationError { message, details }
            }
            AuthError::Password(e) => AppError::Unauthorized(e.to_string()),
            AuthError::Jwt(e) => AppError::Unauthorized(e.to_string()),
            AuthError::Internal(msg) => AppError::internal(msg),
        }
    }
}
