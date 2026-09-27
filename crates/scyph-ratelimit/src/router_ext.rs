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
    /// Applies strict IP rate limiting (5 requests per 2 seconds) to the router.
    fn rate_limit_strict(self) -> Self;

    /// Applies relaxed IP rate limiting (500 requests per 100 milliseconds) to the router.
    fn rate_limit_relaxed(self) -> Self;

    /// Applies custom IP rate limiting configured by [`RateLimitConfig`] to the router.
    fn rate_limit(self, config: RateLimitConfig) -> Self;
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
}
