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
//! ## Quick Example
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
