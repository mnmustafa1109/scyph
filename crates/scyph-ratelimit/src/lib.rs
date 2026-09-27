#![warn(missing_docs)]
//! # Scyph Rate Limit
//!
//! IP-based rate limiting middleware integration for Axum web applications built on `tower-governor`.
//!
//! ## Overview
//!
//! Provides pre-configured, production-ready rate limiting layers, router extensions, and builder configurations:
//! - **[`RateLimitRouterExt`]**: Fluent Axum `Router` extension methods (`.rate_limit_strict()`, `.rate_limit_relaxed()`, `.rate_limit(...)`).
//! - **[`RateLimitConfig`]**: Configuration builder for custom burst sizes and periods.
//! - **[`per_ip_layer`]**: Custom IP rate limiter layer with configurable burst capacity and replenishment interval.
//! - **[`strict_layer`]**: Strict rate limiter (5 requests per 2 seconds) suitable for auth/login endpoints.
//! - **[`relaxed_layer`]**: High-throughput rate limiter (500 requests per 100ms) for high-traffic APIs.
//! - **[`RateLimitError`]**: Strongly-typed error enum for rate limiter initialization.
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph_ratelimit::RateLimitRouterExt;
//!
//! let app = Router::new()
//!     .route("/api/data", get(|| async { "OK" }))
//!     .rate_limit_relaxed();
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
pub use layer::{PeerRateLimitLayer, per_ip_layer, relaxed_layer, strict_layer};

#[doc(inline)]
pub use router_ext::RateLimitRouterExt;
