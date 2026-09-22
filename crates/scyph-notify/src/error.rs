//! Notification error type definitions.

use scyph_core::AppError;

/// Error type for notification operations in `scyph-notify`.
#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    /// Missing or invalid configuration (e.g. environment variables).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Template parsing or rendering failure.
    #[cfg(feature = "email")]
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    /// SMTP transport or delivery failure.
    #[cfg(feature = "email")]
    #[error("SMTP email error: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),

    /// Email address parsing failure.
    #[cfg(feature = "email")]
    #[error("Email address error: {0}")]
    Address(#[from] lettre::address::AddressError),

    /// Email message construction failure.
    #[cfg(feature = "email")]
    #[error("Email build error: {0}")]
    EmailBuild(#[from] lettre::error::Error),

    /// FCM HTTP client failure.
    #[cfg(feature = "fcm")]
    #[error("FCM HTTP request error: {0}")]
    FcmHttp(#[from] reqwest::Error),

    /// FCM authentication token failure.
    #[cfg(feature = "fcm")]
    #[error("FCM authentication error: {0}")]
    FcmAuth(#[from] gcp_auth::Error),

    /// Generic or internal notification error.
    #[error("Notification error: {0}")]
    Internal(String),
}

impl From<NotifyError> for AppError {
    fn from(err: NotifyError) -> Self {
        AppError::internal(err.to_string())
    }
}
