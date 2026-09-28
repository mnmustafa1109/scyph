//! Rate limit configuration builder.

use crate::error::{RateLimitError, Result};
use crate::layer::{PeerRateLimitLayer, per_ip_layer};
use std::env;
use std::time::Duration;

/// Configurable builder for peer-IP rate limiting layers.
///
/// Supports fluent builder configuration, environment variable initialization (`from_env`),
/// and pre-configured presets (`strict` and `relaxed`).
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use scyph_ratelimit::RateLimitConfig;
/// use axum::body::Body;
///
/// let config = RateLimitConfig::new()
///     .with_burst(20)
///     .with_period(Duration::from_secs(1));
///
/// let layer = config.build_layer::<Body>().expect("Layer constructed");
/// ```
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum request capacity allowed in a single burst.
    pub burst_size: u32,
    /// Replenishment period duration.
    pub period: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            burst_size: 10,
            period: Duration::from_secs(1),
        }
    }
}

impl RateLimitConfig {
    /// Constructs a new [`RateLimitConfig`] with default burst (10) and 1-second replenishment period.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads rate limit configuration from environment variables (`RATE_LIMIT_BURST` and `RATE_LIMIT_PERIOD_MS`).
    ///
    /// # Environment Variables
    ///
    /// - `RATE_LIMIT_BURST`: Maximum burst capacity (default: `10`).
    /// - `RATE_LIMIT_PERIOD_MS`: Replenishment period in milliseconds (default: `1000`).
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitError::ConfigurationFailed`] if environment variable values cannot be parsed into integers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_ratelimit::RateLimitConfig;
    /// use axum::body::Body;
    ///
    /// let config = RateLimitConfig::from_env().expect("Valid environment configuration");
    /// let layer = config.build_layer::<Body>().expect("Layer constructed");
    /// ```
    pub fn from_env() -> Result<Self> {
        let burst = env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "10".into())
            .parse::<u32>()
            .map_err(|e| {
                RateLimitError::ConfigurationFailed(format!("Invalid RATE_LIMIT_BURST value: {e}"))
            })?;

        let period_ms = env::var("RATE_LIMIT_PERIOD_MS")
            .unwrap_or_else(|_| "1000".into())
            .parse::<u64>()
            .map_err(|e| {
                RateLimitError::ConfigurationFailed(format!(
                    "Invalid RATE_LIMIT_PERIOD_MS value: {e}"
                ))
            })?;

        Ok(Self {
            burst_size: burst,
            period: Duration::from_millis(period_ms),
        })
    }

    /// Constructs a strict configuration suitable for authentication and payment routes (5 requests per 2s).
    pub fn strict() -> Self {
        Self {
            burst_size: 5,
            period: Duration::from_secs(2),
        }
    }

    /// Constructs a relaxed configuration suitable for public read routes (500 requests per 100ms).
    pub fn relaxed() -> Self {
        Self {
            burst_size: 500,
            period: Duration::from_millis(100),
        }
    }

    /// Sets the maximum burst capacity.
    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self
    }

    /// Sets the replenishment period duration.
    pub fn with_period(mut self, period: Duration) -> Self {
        self.period = period;
        self
    }

    /// Builds a [`PeerRateLimitLayer`] from this configuration.
    pub fn build_layer<B>(&self) -> Result<PeerRateLimitLayer<B>> {
        per_ip_layer(self.burst_size, self.period)
    }

    /// Builds a reverse-proxy-aware [`SmartRateLimitLayer`] from this configuration.
    pub fn build_smart_layer<B>(&self) -> Result<crate::layer::SmartRateLimitLayer<B>> {
        crate::layer::smart_ip_layer(self.burst_size, self.period)
    }
}
