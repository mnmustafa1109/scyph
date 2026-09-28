//! Webhook error types.
//!
//! Defines [`WebhookError`] — all failures that can occur during HMAC-SHA256 webhook signature
//! verification. These map to HTTP client errors (`400 Bad Request`) since they indicate that
//! the incoming request is malformed or untrusted, not a server-side infrastructure failure.

use scyph_core::AppError;

/// Errors encountered during webhook signature verification.
///
/// ## HTTP Mapping
///
/// | Variant | HTTP Status | Meaning |
/// |---------|-------------|---------|
/// | `StaleTimestamp` | 400 | Webhook delivered too late (replay protection) |
/// | `SignatureMismatch` | 400 | Wrong secret or tampered payload |
/// | `InvalidHeaderFormat` | 400 | Malformed signature header |
/// | `HmacKey` | 500 | Bad HMAC key configuration (server fault) |
///
/// # Examples
///
/// ```rust
/// use scyph_utils::webhook::WebhookError;
///
/// let err = WebhookError::SignatureMismatch;
/// assert_eq!(err.to_string(), "Invalid webhook signature");
///
/// let err = WebhookError::StaleTimestamp { tolerance_secs: 300, delta_secs: 450 };
/// assert!(err.to_string().contains("450s"));
/// ```
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
