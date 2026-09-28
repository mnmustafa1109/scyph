#![warn(missing_docs)]
//! # Scyph Rate Limit
//!
//! IP-based and reverse-proxy-aware rate limiting middleware integration for Axum web applications built on `tower-governor`.
//!
//! ## Overview
//!
//! Provides pre-configured, production-ready rate limiting layers, router extensions, and builder configurations:
//! - **[`RateLimitRouterExt`]**: Fluent Axum `Router` extension methods for direct socket and reverse-proxy setups (`.rate_limit_smart_strict()`, `.rate_limit_smart_relaxed()`, `.rate_limit_strict()`, `.rate_limit_relaxed()`).
//! - **[`RateLimitConfig`]**: Configuration builder for custom burst sizes and periods, supporting both standard socket layers ([`RateLimitConfig::build_layer`]) and smart proxy-aware layers ([`RateLimitConfig::build_smart_layer`]).
//! - **Reverse-Proxy-Aware Smart Rate Limiting**: Inspects `CF-Connecting-IP`, `X-Real-IP`, and `X-Forwarded-For` with automatic fallback to socket peer address ([`smart_ip_layer`], [`smart_strict_layer`], [`smart_relaxed_layer`], [`SmartRateLimitLayer`]).
//! - **Standard Direct Socket Layers**: Direct socket address rate limiting ([`per_ip_layer`], [`strict_layer`], [`relaxed_layer`], [`PeerRateLimitLayer`]).
//! - **[`strict_layer`] / [`smart_strict_layer`]**: Strict rate limiter (5 requests per 2 seconds) suitable for auth/login endpoints.
//! - **[`relaxed_layer`] / [`smart_relaxed_layer`]**: High-throughput rate limiter (500 requests per 100ms) for high-traffic APIs.
//! - **[`RateLimitError`]**: Strongly-typed error enum for rate limiter initialization.
//!
//! ## Choosing a Layer Variant
//!
//! | Deployment | Recommended Layer | Key Extractor |
//! |---|---|---|
//! | Direct (no proxy) | [`PeerRateLimitLayer`] / [`strict_layer`] / [`relaxed_layer`] | `PeerIpKeyExtractor` (socket addr) |
//! | Behind Nginx/Cloudflare/ALB | [`SmartRateLimitLayer`] / [`smart_strict_layer`] / [`smart_relaxed_layer`] | `SmartIpKeyExtractor` (headers + fallback) |
//!
//! When using [`PeerRateLimitLayer`], the Axum server **must** be started with
//! `.into_make_service_with_connect_info::<SocketAddr>()` so that the socket peer address
//! is available in request extensions. Failing to do so results in all requests being treated
//! as coming from the same IP, effectively applying a global rate limit.
//!
//! [`SmartRateLimitLayer`] does not require `ConnectInfo` and is therefore the safer default
//! for most production deployments.
//!
//! ## Quick Start — Reverse Proxy Deployment
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph_ratelimit::RateLimitRouterExt;
//!
//! // Behind reverse proxy (Cloudflare, Nginx, ALB):
//! let app = Router::new()
//!     .route("/api/data", get(|| async { "OK" }))
//!     .rate_limit_smart_relaxed();
//! ```
//!
//! ## Quick Start — Direct Socket Deployment
//!
//! ```rust,ignore
//! use axum::{Router, routing::post};
//! use scyph_ratelimit::RateLimitRouterExt;
//! use std::net::SocketAddr;
//!
//! let app = Router::new()
//!     .route("/login", post(|| async { "login" }))
//!     .rate_limit_strict();
//!
//! // IMPORTANT: ConnectInfo MUST be provided for PeerIpKeyExtractor to work.
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//! axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
//!     .await
//!     .unwrap();
//! ```
//!
//! ## Quick Start — Custom Configuration
//!
//! ```rust,ignore
//! use axum::{Router, routing::get};
//! use scyph_ratelimit::{RateLimitConfig, RateLimitRouterExt};
//! use std::time::Duration;
//!
//! // Load from environment or fall back to defaults
//! let config = RateLimitConfig::from_env()
//!     .unwrap_or_else(|_| RateLimitConfig::new().with_burst(50).with_period(Duration::from_secs(1)));
//!
//! let app = Router::new()
//!     .route("/api/v1/feed", get(|| async { "feed" }))
//!     .rate_limit_smart(config);
//! ```

/// Rate limit configuration builder.
pub mod config;

/// Rate limit error types.
pub mod error;

/// Pre-configured rate limiting layers and key extractors.
pub mod layer;

/// Axum Router extension trait.
pub mod router_ext;

#[doc(inline)]
pub use config::RateLimitConfig;

#[doc(inline)]
pub use error::{RateLimitError, Result};

#[doc(inline)]
pub use layer::{
    PeerRateLimitLayer, SmartRateLimitLayer, per_ip_layer, relaxed_layer, smart_ip_layer,
    smart_relaxed_layer, smart_strict_layer, strict_layer,
};

#[doc(inline)]
pub use router_ext::RateLimitRouterExt;
