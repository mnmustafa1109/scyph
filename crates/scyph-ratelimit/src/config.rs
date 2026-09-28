//! Rate limit configuration builder.

use crate::error::{RateLimitError, Result};
use crate::layer::{PeerRateLimitLayer, SmartRateLimitLayer, per_ip_layer, smart_ip_layer};
use std::env;
use std::time::Duration;

/// Configurable builder for peer-IP rate limiting layers.
///
/// Supports fluent builder configuration, environment variable initialization (`from_env`),
/// and pre-configured presets (`strict` and `relaxed`).
///
/// # Architecture
///
/// `RateLimitConfig` acts as a configuration value object that builds Tower middleware layers
/// via [`build_layer`](Self::build_layer) (socket peer IP) or [`build_smart_layer`](Self::build_smart_layer)
/// (proxy-aware header inspection). Both methods delegate to [`per_ip_layer`] and [`smart_ip_layer`]
/// respectively, encapsulating `tower-governor` `GovernorConfigBuilder` construction.
///
/// # Preset Comparison
///
/// | Preset | Burst | Period | Use Case |
/// |---|---|---|---|
/// | [`RateLimitConfig::strict`] | 5 | 2 s | Auth, login, password reset, payments |
/// | [`RateLimitConfig::relaxed`] | 500 | 100 ms | Public APIs, CDN-backed feeds |
/// | [`RateLimitConfig::new`] | 10 | 1 s | General-purpose default |
///
/// # Full Router Integration Example
///
/// ```rust,ignore
/// use axum::{Router, routing::{get, post}};
/// use scyph_ratelimit::{RateLimitConfig, RateLimitRouterExt};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     // Auth endpoints: extremely strict to prevent brute-force
///     let auth_routes = Router::new()
///         .route("/login", post(|| async { "login" }))
///         .route("/register", post(|| async { "register" }))
///         .route("/forgot-password", post(|| async { "forgot" }))
///         .rate_limit_smart_strict();
///
///     // API endpoints: relaxed for high-throughput reads
///     let api_routes = Router::new()
///         .route("/api/feed", get(|| async { "feed" }))
///         .route("/api/search", get(|| async { "search" }))
///         .rate_limit_smart_relaxed();
///
///     // Custom config loaded from environment at startup
///     let upload_config = RateLimitConfig::from_env()
///         .unwrap_or_else(|_| {
///             RateLimitConfig::new()
///                 .with_burst(10)
///                 .with_period(Duration::from_secs(60))
///         });
///
///     let upload_routes = Router::new()
///         .route("/api/upload", post(|| async { "upload" }))
///         .rate_limit_smart(upload_config);
///
///     let app = Router::new()
///         .merge(auth_routes)
///         .merge(api_routes)
///         .merge(upload_routes);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
///
/// # Simple Builder Example
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
    ///
    /// After a burst is exhausted, new tokens are replenished at the rate of one per `period / burst_size`.
    /// Must be greater than 0.
    pub burst_size: u32,
    /// Token replenishment period duration.
    ///
    /// Combined with `burst_size`, determines the steady-state request rate.
    /// For example, `burst_size = 500` with `period = 100ms` allows up to 500 req/100ms.
    /// Must be at least 1 millisecond.
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
    ///
    /// Use [`with_burst`](Self::with_burst) and [`with_period`](Self::with_period) to customize.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads rate limit configuration from environment variables (`RATE_LIMIT_BURST` and `RATE_LIMIT_PERIOD_MS`).
    ///
    /// # Environment Variables
    ///
    /// | Variable | Description | Default |
    /// |---|---|---|
    /// | `RATE_LIMIT_BURST` | Maximum burst capacity (requests per window) | `10` |
    /// | `RATE_LIMIT_PERIOD_MS` | Replenishment period in milliseconds | `1000` |
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
    ///
    /// This preset limits each IP to 5 requests every 2 seconds, making brute-force and
    /// credential-stuffing attacks impractical. Appropriate for `/login`, `/register`,
    /// `/forgot-password`, and payment processing endpoints.
    pub fn strict() -> Self {
        Self {
            burst_size: 5,
            period: Duration::from_secs(2),
        }
    }

    /// Constructs a relaxed configuration suitable for public read routes (500 requests per 100ms).
    ///
    /// This preset is appropriate for high-throughput public APIs, CDN-backed content feeds,
    /// or search endpoints where legitimate clients may issue rapid successive requests.
    pub fn relaxed() -> Self {
        Self {
            burst_size: 500,
            period: Duration::from_millis(100),
        }
    }

    /// Sets the maximum burst capacity.
    ///
    /// Must be greater than 0. Panics at layer construction time if set to 0.
    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self
    }

    /// Sets the replenishment period duration.
    ///
    /// Must be at least 1 millisecond. Panics at layer construction time if set to zero.
    pub fn with_period(mut self, period: Duration) -> Self {
        self.period = period;
        self
    }

    /// Builds a [`PeerRateLimitLayer`] from this configuration using direct socket peer IP extraction.
    ///
    /// **Note:** The Axum server must be started with
    /// `.into_make_service_with_connect_info::<SocketAddr>()` for peer IP extraction to work.
    /// For reverse proxy deployments, use [`build_smart_layer`](Self::build_smart_layer) instead.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitError::ConfigurationFailed`] if `burst_size` is 0 or `period` is sub-millisecond.
    pub fn build_layer<B>(&self) -> Result<PeerRateLimitLayer<B>> {
        per_ip_layer(self.burst_size, self.period)
    }

    /// Builds a reverse-proxy-aware [`SmartRateLimitLayer`] from this configuration.
    ///
    /// The smart layer inspects `CF-Connecting-IP`, `X-Real-IP`, and `X-Forwarded-For` headers
    /// in priority order, falling back to the socket peer address when no proxy header is present.
    /// This is the recommended layer for any deployment behind Nginx, Cloudflare, or AWS ALB.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimitError::ConfigurationFailed`] if `burst_size` is 0 or `period` is sub-millisecond.
    pub fn build_smart_layer<B>(&self) -> Result<SmartRateLimitLayer<B>> {
        smart_ip_layer(self.burst_size, self.period)
    }
}
