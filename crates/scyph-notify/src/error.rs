//! Notification error type definitions for `scyph-notify`.
//!
//! Categorizes all possible failures occurring during email compilation, SMTP delivery,
//! FCM push API calls, template rendering, and missing configuration variables.

use scyph_core::AppError;

/// Error type for all notification services and utilities in `scyph-notify`.
#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    /// Missing, empty, or unparseable configuration settings or environment variables (e.g. `SMTP_HOST`, `FCM_PROJECT_ID`).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Tera template parsing, compilation, or variable context rendering failure.
    #[cfg(feature = "email")]
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    /// SMTP transport, connection, authentication, or network transmission failure.
    #[cfg(feature = "email")]
    #[error("SMTP email error: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),

    /// Email recipient or sender address parsing failure (e.g., malformed email address string).
    #[cfg(feature = "email")]
    #[error("Email address error: {0}")]
    Address(#[from] lettre::address::AddressError),

    /// Email message envelope, header, or body formatting failure during `lettre` message construction.
    #[cfg(feature = "email")]
    #[error("Email build error: {0}")]
    EmailBuild(#[from] lettre::error::Error),

    /// HTTP client transport, connection, or response status error during FCM REST API requests.
    #[cfg(feature = "fcm")]
    #[error("FCM HTTP request error: {0}")]
    FcmHttp(#[from] reqwest::Error),

    /// Service account authentication or OAuth2 access token acquisition failure for Google Cloud FCM API.
    #[cfg(feature = "fcm")]
    #[error("FCM authentication error: {0}")]
    FcmAuth(#[from] gcp_auth::Error),

    /// Internal or generic notification processing error.
    #[error("Notification error: {0}")]
    Internal(String),
}

impl From<NotifyError> for AppError {
    /// Converts a [`NotifyError`] into an HTTP 500 RFC 7807 [`AppError`] for Axum API responses.
    fn from(err: NotifyError) -> Self {
        AppError::internal(err.to_string())
    }
}
