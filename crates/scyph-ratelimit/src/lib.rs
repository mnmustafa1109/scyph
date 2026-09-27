#![warn(missing_docs)]
//! # Scyph Rate Limit
//!
//! IP-based rate limiting middleware integration for Axum web applications built on `tower-governor`.
//!
//! ## Overview
//!
//! Provides pre-configured, production-ready rate limiting layers:
//! - [`per_ip_layer`]: Custom IP rate limiter with configurable burst capacity and replenishment interval.
//! - [`strict_layer`]: Strict rate limiter (5 requests per 2 seconds) suitable for auth/login endpoints.
//! - [`relaxed_layer`]: High-throughput rate limiter (500 requests per 100ms) for high-traffic APIs.
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph_ratelimit::relaxed_layer;
//!
//! let app = Router::new()
//!     .route("/api/data", get(|| async { "OK" }))
//!     .layer(relaxed_layer().expect("Rate limit layer initialized"));
//! ```

/// Pre-configured rate limiting layers and key extractors.
pub mod layer;

#[doc(inline)]
pub use layer::{per_ip_layer, relaxed_layer, strict_layer, PeerRateLimitLayer};
