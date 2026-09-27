//! Rate limit configuration builder.

use std::time::Duration;
use crate::error::Result;
use crate::layer::{per_ip_layer, PeerRateLimitLayer};

/// Configurable builder for peer-IP rate limiting layers.
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
    /// Constructs a new [`RateLimitConfig`] with default burst (10) and 1-second period.
    pub fn new() -> Self {
        Self::default()
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
}
