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
/// Uses `PeerIpKeyExtractor` which reads the remote socket address from Axum's
/// [`ConnectInfo<SocketAddr>`](axum::extract::ConnectInfo) request extension.
///
/// ## ⚠ Important: ConnectInfo Requirement
///
/// For this layer to extract per-IP keys correctly, the Axum server **must** be initialized with:
///
/// ```rust,ignore
/// use std::net::SocketAddr;
///
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
/// axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
///     .await
///     .unwrap();
/// ```
///
/// If `ConnectInfo` is not set up, all requests appear to come from the same IP address,
/// effectively turning this into a global rate limiter. For deployments behind a reverse
/// proxy, prefer [`SmartRateLimitLayer`] which reads from forwarding headers.
pub type PeerRateLimitLayer<B = Body> = GovernorLayer<
    PeerIpKeyExtractor,
    NoOpMiddleware<<DefaultClock as governor::clock::Clock>::Instant>,
    B,
>;

/// Type alias for the smart IP rate limiting layer for reverse-proxy deployments.
///
/// Uses `SmartIpKeyExtractor` which inspects the following headers in priority order,
/// falling back to the socket peer address when none are present:
///
/// 1. `CF-Connecting-IP` (Cloudflare)
/// 2. `X-Real-IP` (Nginx)
/// 3. `X-Forwarded-For` (first entry in the list)
/// 4. Socket peer address (direct connections)
///
/// This type alias does **not** require `ConnectInfo<SocketAddr>` to be configured on the
/// Axum server, making it suitable for deployments behind any reverse proxy.
///
/// Prefer this over [`PeerRateLimitLayer`] whenever running behind Nginx, Cloudflare, AWS ALB,
/// or any other load balancer that injects IP forwarding headers.
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
    let period_ms = period.as_millis() as u64;
    if period_ms == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Replenishment period must be at least 1 millisecond".to_string(),
        ));
    }

    let config = GovernorConfigBuilder::default()
        .key_extractor(PeerIpKeyExtractor)
        .per_millisecond(period_ms)
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
/// * `period` - Replenishment duration interval (must be >= 1ms).
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
    let period_ms = period.as_millis() as u64;
    if period_ms == 0 {
        return Err(RateLimitError::ConfigurationFailed(
            "Replenishment period must be at least 1 millisecond".to_string(),
        ));
    }

    let config = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_millisecond(period_ms)
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
///
/// Identical policy to [`strict_layer`] but uses [`SmartIpKeyExtractor`] to correctly extract
/// the real client IP behind reverse proxies. Does not require `ConnectInfo<SocketAddr>`.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
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
///
/// Identical policy to [`relaxed_layer`] but uses [`SmartIpKeyExtractor`] to correctly extract
/// the real client IP behind reverse proxies. Does not require `ConnectInfo<SocketAddr>`.
///
/// # Errors
///
/// Returns [`RateLimitError::ConfigurationFailed`] if layer construction fails.
pub fn smart_relaxed_layer<B>() -> Result<SmartRateLimitLayer<B>, RateLimitError> {
    smart_ip_layer(500, Duration::from_millis(100))
}
