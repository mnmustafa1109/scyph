#![warn(missing_docs)]
//! # Scyph Utils
//!
//! `scyph-utils` provides essential utility primitives for Axum backend applications:
//! - **[`idempotency`]**: Redis-backed API request deduplication, configuration, and response caching.
//! - **[`pagination`]**: Base64 URL-safe UUID cursor pagination helpers and Axum query parameter extractors.
//! - **[`webhook`]**: HMAC-SHA256 webhook signature verification, header parsing, and constant-time comparison.
//! - **[`worker`]**: Periodic background worker loops with panic isolation and graceful shutdown integration.

/// Redis-backed API idempotency store and configuration.
pub mod idempotency;

/// Base64 URL-safe UUID cursor pagination.
pub mod pagination;

/// HMAC-SHA256 webhook verification.
pub mod webhook;

/// Background task worker loop manager.
pub mod worker;

#[doc(inline)]
pub use idempotency::{IdempotencyCheck, IdempotencyConfig, IdempotencyError, IdempotencyStore};

#[doc(inline)]
pub use pagination::{Cursor, PageParams, PaginationError};

#[doc(inline)]
pub use webhook::{WebhookError, verify_webhook, verify_webhook_header};

#[doc(inline)]
pub use worker::{WorkerError, spawn_worker, spawn_worker_cancel};
