#![warn(missing_docs)]
//! # Scyph Utils
//!
//! `scyph-utils` provides essential utility primitives for Axum backend applications:
//! - **[`idempotency`]**: Redis-backed API request deduplication and response caching.
//! - **[`pagination`]**: Base64 URL-safe UUID cursor pagination helpers and Axum query parameter extractors.
//! - **[`webhook`]**: HMAC-SHA256 webhook signature verification, replay attack prevention, and constant-time string comparison.
//! - **[`worker`]**: Periodic background worker loops with panic isolation and graceful shutdown integration.
//! - **[`config`]**: Environment configuration structures and default loaders.
//! - **[`error`]**: Strongly typed utility error definitions (`UtilsError`).

/// Configuration loaders and builder structures.
pub mod config;

/// Utility error types and RFC 7807 problem details implementations.
pub mod error;

/// Redis-backed API idempotency store.
pub mod idempotency;

/// Base64 URL-safe UUID cursor pagination.
pub mod pagination;

/// HMAC-SHA256 webhook verification.
pub mod webhook;

/// Background task worker loop manager.
pub mod worker;

#[doc(inline)]
pub use config::IdempotencyConfig;

#[doc(inline)]
pub use error::{Result, UtilsError};

#[doc(inline)]
pub use idempotency::{IdempotencyCheck, IdempotencyStore};

#[doc(inline)]
pub use pagination::{Cursor, PageParams};

#[doc(inline)]
pub use webhook::{verify_webhook, verify_webhook_header};

#[doc(inline)]
pub use worker::{spawn_worker, spawn_worker_cancel};
