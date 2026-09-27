//! Error types for rate limit configuration and layer construction.

use thiserror::Error;

/// Error type for rate limit layer creation and governor configuration errors.
#[derive(Debug, Error)]
pub enum RateLimitError {
    /// Failed to build governor rate limit configuration.
    #[error("Failed to construct governor rate limit configuration: {0}")]
    ConfigurationFailed(String),
}

/// Standardized result type alias for rate limit operations.
pub type Result<T, E = RateLimitError> = std::result::Result<T, E>;
