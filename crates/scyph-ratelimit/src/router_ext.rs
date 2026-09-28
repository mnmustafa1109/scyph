//! Axum Router extension trait for fluent rate limit layer registration.

use crate::config::RateLimitConfig;
use axum::Router;

/// Extension trait for [`axum::Router`] providing fluent rate limiting middleware registration.
///
/// # Examples
///
/// ```rust,ignore
/// use axum::{routing::get, Router};
/// use scyph_ratelimit::RateLimitRouterExt;
///
/// let app = Router::new()
///     .route("/login", get(|| async { "login" }))
///     .rate_limit_strict();
/// ```
pub trait RateLimitRouterExt<S = ()> {
    /// Applies strict peer-IP rate limiting (5 requests per 2 seconds) to the router.
    ///
    /// Note: Requires `into_make_service_with_connect_info::<SocketAddr>()` on the Axum server.
    fn rate_limit_strict(self) -> Self;

    /// Applies relaxed peer-IP rate limiting (500 requests per 100 milliseconds) to the router.
    ///
    /// Note: Requires `into_make_service_with_connect_info::<SocketAddr>()` on the Axum server.
    fn rate_limit_relaxed(self) -> Self;

    /// Applies custom peer-IP rate limiting configured by [`RateLimitConfig`] to the router.
    fn rate_limit(self, config: RateLimitConfig) -> Self;

    /// Applies proxy-aware smart IP rate limiting (5 requests per 2 seconds) to the router.
    ///
    /// Suitable for applications behind reverse proxies (NGINX, Cloudflare, AWS ALB).
    fn rate_limit_smart_strict(self) -> Self;

    /// Applies proxy-aware smart IP rate limiting (500 requests per 100 milliseconds) to the router.
    ///
    /// Suitable for applications behind reverse proxies (NGINX, Cloudflare, AWS ALB).
    fn rate_limit_smart_relaxed(self) -> Self;

    /// Applies custom proxy-aware smart IP rate limiting configured by [`RateLimitConfig`] to the router.
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
