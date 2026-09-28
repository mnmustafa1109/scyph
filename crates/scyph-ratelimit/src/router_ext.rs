//! Axum Router extension trait for fluent rate limit layer registration.

use crate::config::RateLimitConfig;
use axum::Router;

/// Extension trait for [`axum::Router`] providing fluent rate limiting middleware registration.
///
/// This trait provides convenient one-liner methods for attaching pre-configured or custom
/// rate limiting middleware to Axum routers. All methods consume and return the router,
/// enabling method chaining.
///
/// ## Direct vs Smart Methods
///
/// Methods without the `_smart` suffix use [`PeerIpKeyExtractor`](tower_governor::key_extractor::PeerIpKeyExtractor)
/// (direct socket address). These **require** the Axum server to be started with
/// `.into_make_service_with_connect_info::<SocketAddr>()`.
///
/// Methods with the `_smart` suffix use [`SmartIpKeyExtractor`](tower_governor::key_extractor::SmartIpKeyExtractor)
/// which reads from proxy headers (`CF-Connecting-IP`, `X-Real-IP`, `X-Forwarded-For`). These
/// are the recommended default for any deployment behind a reverse proxy.
///
/// ## Rate Limits for Auth vs API Endpoints
///
/// ```rust,ignore
/// use axum::{Router, routing::{get, post}};
/// use scyph_ratelimit::{RateLimitConfig, RateLimitRouterExt};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     // Auth routes: strict 5 req/2s to prevent brute-force & credential stuffing
///     let auth_router = Router::new()
///         .route("/auth/login", post(|| async { "login" }))
///         .route("/auth/register", post(|| async { "register" }))
///         .route("/auth/forgot-password", post(|| async { "forgot-password" }))
///         .rate_limit_smart_strict();
///
///     // High-throughput API routes: relaxed 500 req/100ms
///     let api_router = Router::new()
///         .route("/api/posts", get(|| async { "posts" }))
///         .route("/api/search", get(|| async { "search" }))
///         .rate_limit_smart_relaxed();
///
///     // Upload routes: custom 10 req/min from env or hardcoded fallback
///     let upload_config = RateLimitConfig::from_env()
///         .unwrap_or_else(|_| {
///             RateLimitConfig::new()
///                 .with_burst(10)
///                 .with_period(Duration::from_secs(60))
///         });
///     let upload_router = Router::new()
///         .route("/api/files/upload", post(|| async { "upload" }))
///         .rate_limit_smart(upload_config);
///
///     // Compose all routers into the application
///     let app = Router::new()
///         .merge(auth_router)
///         .merge(api_router)
///         .merge(upload_router);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
///
/// ## Simple Usage
///
/// ```rust,ignore
/// use axum::{routing::get, Router};
/// use scyph_ratelimit::RateLimitRouterExt;
///
/// let app = Router::new()
///     .route("/login", axum::routing::post(|| async { "login" }))
///     .rate_limit_strict();
/// ```
pub trait RateLimitRouterExt<S = ()> {
    /// Applies strict peer-IP rate limiting (5 requests per 2 seconds) to the router.
    ///
    /// Suitable for sensitive endpoints such as authentication, password reset, or payment gateways.
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails (should never occur with valid preset parameters).
    ///
    /// # Note
    ///
    /// Requires the Axum server to be started with
    /// `.into_make_service_with_connect_info::<SocketAddr>()` for correct per-IP limiting.
    /// For reverse proxy deployments, use [`rate_limit_smart_strict`](Self::rate_limit_smart_strict) instead.
    fn rate_limit_strict(self) -> Self;

    /// Applies relaxed peer-IP rate limiting (500 requests per 100 milliseconds) to the router.
    ///
    /// Suitable for high-throughput public API endpoints or WebSocket handshake routes.
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails (should never occur with valid preset parameters).
    ///
    /// # Note
    ///
    /// Requires the Axum server to be started with
    /// `.into_make_service_with_connect_info::<SocketAddr>()` for correct per-IP limiting.
    /// For reverse proxy deployments, use [`rate_limit_smart_relaxed`](Self::rate_limit_smart_relaxed) instead.
    fn rate_limit_relaxed(self) -> Self;

    /// Applies custom peer-IP rate limiting configured by [`RateLimitConfig`] to the router.
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails due to invalid `burst_size` (0) or `period` (< 1ms).
    fn rate_limit(self, config: RateLimitConfig) -> Self;

    /// Applies proxy-aware smart IP rate limiting (5 requests per 2 seconds) to the router.
    ///
    /// Suitable for applications behind reverse proxies (Nginx, Cloudflare, AWS ALB).
    /// Inspects `CF-Connecting-IP`, `X-Real-IP`, and `X-Forwarded-For` headers before
    /// falling back to the socket peer address.
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails (should never occur with valid preset parameters).
    fn rate_limit_smart_strict(self) -> Self;

    /// Applies proxy-aware smart IP rate limiting (500 requests per 100 milliseconds) to the router.
    ///
    /// Suitable for high-throughput applications behind reverse proxies (Nginx, Cloudflare, AWS ALB).
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails (should never occur with valid preset parameters).
    fn rate_limit_smart_relaxed(self) -> Self;

    /// Applies custom proxy-aware smart IP rate limiting configured by [`RateLimitConfig`] to the router.
    ///
    /// # Panics
    ///
    /// Panics if layer construction fails due to invalid `burst_size` (0) or `period` (< 1ms).
    fn rate_limit_smart(self, config: RateLimitConfig) -> Self;
}

impl<S> RateLimitRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn rate_limit_strict(self) -> Self {
        let layer = RateLimitConfig::strict()
            .build_layer()
            .expect("Strict rate limit layer configuration");
        self.layer(layer)
    }

    fn rate_limit_relaxed(self) -> Self {
        let layer = RateLimitConfig::relaxed()
            .build_layer()
            .expect("Relaxed rate limit layer configuration");
        self.layer(layer)
    }

    fn rate_limit(self, config: RateLimitConfig) -> Self {
        let layer = config
            .build_layer()
            .expect("Rate limit layer configuration");
        self.layer(layer)
    }

    fn rate_limit_smart_strict(self) -> Self {
        let layer = RateLimitConfig::strict()
            .build_smart_layer()
            .expect("Strict smart rate limit layer configuration");
        self.layer(layer)
    }

    fn rate_limit_smart_relaxed(self) -> Self {
        let layer = RateLimitConfig::relaxed()
            .build_smart_layer()
            .expect("Relaxed smart rate limit layer configuration");
        self.layer(layer)
    }

    fn rate_limit_smart(self, config: RateLimitConfig) -> Self {
        let layer = config
            .build_smart_layer()
            .expect("Smart rate limit layer configuration");
        self.layer(layer)
    }
}
