//! Error types for rate limit configuration and layer construction.

use thiserror::Error;

/// Error type for rate limit layer creation and governor configuration errors.
///
/// This error is returned during application startup when building a rate limiting layer
/// from invalid parameters (e.g., zero burst size, sub-millisecond period, or unparseable
/// environment variables). It is not returned during per-request rate limit enforcement;
/// `tower-governor` handles per-request throttling internally, returning HTTP 429 responses.
///
/// # Error Recovery
///
/// `RateLimitError` is typically encountered at startup time. The recommended pattern is:
///
/// ```rust
/// use scyph_ratelimit::{RateLimitConfig, RateLimitError};
/// use axum::body::Body;
///
/// fn build_layer() -> Result<(), RateLimitError> {
///     let config = RateLimitConfig::from_env()?;
///     let _layer = config.build_smart_layer::<Body>()?;
///     Ok(())
/// }
///
/// // Or with a fallback:
/// use std::time::Duration;
///
/// let config = RateLimitConfig::from_env()
///     .unwrap_or_else(|err| {
///         eprintln!("Failed to load rate limit config from env: {err}. Using defaults.");
///         RateLimitConfig::new()
///     });
/// ```
#[derive(Debug, Error)]
pub enum RateLimitError {
    /// Failed to build governor rate limit configuration.
    ///
    /// This variant is produced when:
    /// - `burst_size` is 0
    /// - `period` is shorter than 1 millisecond
    /// - An environment variable (`RATE_LIMIT_BURST` or `RATE_LIMIT_PERIOD_MS`) contains
    ///   a non-integer value
    /// - `GovernorConfigBuilder::finish()` returns `None` for any other reason
    ///
    /// The inner `String` contains a human-readable description of the failure cause.
    #[error("Failed to construct governor rate limit configuration: {0}")]
    ConfigurationFailed(String),
}

/// Standardized result type alias for rate limit operations.
///
/// Equivalent to `std::result::Result<T, RateLimitError>`.
pub type Result<T, E = RateLimitError> = std::result::Result<T, E>;
