//! Rate limiting middleware layers and governor configurations.

use std::time::Duration;
use axum::body::Body;
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;
use tower_governor::GovernorLayer;

use crate::error::RateLimitError;

/// Type alias for the standard peer-IP rate limiting layer.
pub type PeerRateLimitLayer<B = Body> =
    GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>, B>;

/// Creates a peer-IP rate limiting layer with specified burst capacity and replenishment interval.
///
/// # Arguments
///
/// * `burst` - Maximum number of allowed requests in a single burst.
/// * `period` - Replenishment duration interval.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if `GovernorConfigBuilder` fails to construct a valid configuration.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use scyph_ratelimit::per_ip_layer;
/// use axum::body::Body;
///
/// let layer = per_ip_layer::<Body>(10, Duration::from_secs(1)).expect("valid layer");
/// ```
pub fn per_ip_layer<B>(burst: u32, period: Duration) -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    let period_ms = period.as_millis() as u64;
    let config = GovernorConfigBuilder::default()
        .per_millisecond(period_ms)
        .burst_size(burst)
        .finish()
        .ok_or_else(|| RateLimitError::ConfigurationFailed("Failed to build governor configuration".to_string()))?;

    Ok(GovernorLayer::new(config))
}

/// Creates a strict rate limiting layer (5 requests per 2 seconds).
///
/// Useful for sensitive endpoints such as authentication, password reset, or payment gateways.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
///
/// # Examples
///
/// ```rust
/// use scyph_ratelimit::strict_layer;
/// use axum::body::Body;
///
/// let layer = strict_layer::<Body>().expect("strict rate limiter initialized");
/// ```
pub fn strict_layer<B>() -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    per_ip_layer(5, Duration::from_secs(2))
}

/// Creates a relaxed rate limiting layer (500 requests per 100 milliseconds).
///
/// Useful for high-throughput API routes, public read-only endpoints, or WebSocket handshakes.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
///
/// # Examples
///
/// ```rust
/// use scyph_ratelimit::relaxed_layer;
/// use axum::body::Body;
///
/// let layer = relaxed_layer::<Body>().expect("relaxed rate limiter initialized");
/// ```
pub fn relaxed_layer<B>() -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    per_ip_layer(500, Duration::from_millis(100))
}
