//! Rate limiting middleware layers and governor configurations.

use axum::body::Body;
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use std::time::Duration;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::{PeerIpKeyExtractor, SmartIpKeyExtractor};

use crate::error::RateLimitError;

/// Type alias for the standard peer-IP rate limiting layer.
///
/// **Note:** Requires `ConnectInfo<SocketAddr>` to be configured on Axum via
/// `into_make_service_with_connect_info::<SocketAddr>()`. For reverse proxy deployments,
/// prefer [`SmartRateLimitLayer`].
pub type PeerRateLimitLayer<B = Body> = GovernorLayer<
    PeerIpKeyExtractor,
    NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
    B,
>;

/// Type alias for smart IP rate limiting layer that handles reverse proxies (extracts client IP from `Forwarded` or `X-Forwarded-For`).
pub type SmartRateLimitLayer<B = Body> = GovernorLayer<
    SmartIpKeyExtractor,
    NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
    B,
>;

/// Creates a peer-IP rate limiting layer with specified burst capacity and replenishment interval.
///
/// **Note:** Your Axum server listener must be initialized with
/// `.into_make_service_with_connect_info::<SocketAddr>()` for peer IP extraction to succeed.
/// Behind reverse proxies or load balancers, use [`smart_ip_layer`] instead.
///
/// # Arguments
///
/// * `burst` - Maximum number of allowed requests in a single burst (must be > 0).
/// * `period` - Replenishment duration interval (must be >= 1ms).
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if inputs are invalid or if `GovernorConfigBuilder` fails.
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
pub fn per_ip_layer<B>(
    burst: u32,
    period: Duration,
) -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    if burst == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Burst size must be greater than 0".to_string(),
        ));
    }
    let period_nanos = period.as_nanos();
    if period_nanos == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Replenishment period must be at least 1 nanosecond".to_string(),
        ));
    }
    // Calculate per-token replenishment interval (period / burst) so that `burst` tokens
    // are replenished over `period` duration.
    let per_token_nanos = (period_nanos / burst as u128).max(1) as u64;

    let config = GovernorConfigBuilder::default()
        .key_extractor(PeerIpKeyExtractor)
        .per_nanosecond(per_token_nanos)
        .burst_size(burst)
        .finish()
        .ok_or_else(|| {
            RateLimitError::ConfigurationFailed(
                "Failed to build governor configuration".to_string(),
            )
        })?;

    Ok(GovernorLayer::new(config))
}

/// Creates a reverse-proxy-aware smart IP rate limiting layer with specified burst and interval.
///
/// Extracts the real client IP from standard proxy headers (`Forwarded`, `X-Forwarded-For`),
/// falling back to the peer socket IP if headers are absent.
///
/// # Arguments
///
/// * `burst` - Maximum number of allowed requests in a single burst (must be > 0).
/// * `period` - Replenishment duration interval for replenishing `burst` requests.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if inputs are invalid or if `GovernorConfigBuilder` fails.
pub fn smart_ip_layer<B>(
    burst: u32,
    period: Duration,
) -> Result<SmartRateLimitLayer<B>, RateLimitError> {
    if burst == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Burst size must be greater than 0".to_string(),
        ));
    }
    let period_nanos = period.as_nanos();
    if period_nanos == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Replenishment period must be at least 1 nanosecond".to_string(),
        ));
    }
    // Calculate per-token replenishment interval (period / burst)
    let per_token_nanos = (period_nanos / burst as u128).max(1) as u64;

    let config = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_nanosecond(per_token_nanos)
        .burst_size(burst)
        .finish()
        .ok_or_else(|| {
            RateLimitError::ConfigurationFailed(
                "Failed to build governor configuration".to_string(),
            )
        })?;

    Ok(GovernorLayer::new(config))
}

/// Creates a strict peer-IP rate limiting layer (5 requests per 2 seconds).
///
/// Useful for sensitive endpoints such as authentication, password reset, or payment gateways.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
pub fn strict_layer<B>() -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    per_ip_layer(5, Duration::from_secs(2))
}

/// Creates a strict smart rate limiting layer for proxy environments (5 requests per 2 seconds).
pub fn smart_strict_layer<B>() -> Result<SmartRateLimitLayer<B>, RateLimitError> {
    smart_ip_layer(5, Duration::from_secs(2))
}

/// Creates a relaxed peer-IP rate limiting layer (500 requests per 100 milliseconds).
///
/// Useful for high-throughput API routes, public read-only endpoints, or WebSocket handshakes.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
pub fn relaxed_layer<B>() -> Result<PeerRateLimitLayer<B>, RateLimitError> {
    per_ip_layer(500, Duration::from_millis(100))
}

/// Creates a relaxed smart rate limiting layer for proxy environments (500 requests per 100 milliseconds).
pub fn smart_relaxed_layer<B>() -> Result<SmartRateLimitLayer<B>, RateLimitError> {
    smart_ip_layer(500, Duration::from_millis(100))
}
