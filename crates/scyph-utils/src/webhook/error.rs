//! Webhook error types.

use scyph_core::AppError;

/// Errors encountered during webhook signature verification.
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    /// Webhook timestamp is outside permitted tolerance window.
    #[error("Stale webhook timestamp: delta {delta_secs}s exceeds tolerance {tolerance_secs}s")]
    StaleTimestamp {
        /// Configured tolerance threshold in seconds.
        tolerance_secs: i64,
        /// Actual difference between server time and webhook timestamp.
        delta_secs: i64,
    },

    /// HMAC webhook signature verification mismatch.
    #[error("Invalid webhook signature")]
    SignatureMismatch,

    /// Malformed or invalid webhook signature header format.
    #[error("Invalid webhook header format: {0}")]
    InvalidHeaderFormat(String),

    /// Key initialization error for HMAC cryptography.
    #[error("HMAC secret key error: {0}")]
    HmacKey(String),
}

impl From<WebhookError> for AppError {
    fn from(err: WebhookError) -> Self {
        match err {
            WebhookError::StaleTimestamp { .. }
            | WebhookError::SignatureMismatch
            | WebhookError::InvalidHeaderFormat(_) => AppError::BadRequest(err.to_string()),
            WebhookError::HmacKey(msg) => AppError::internal(msg),
        }
    }
}
