#![warn(missing_docs)]
//! # Scyph Utils
//!
//! `scyph-utils` provides essential utility primitives for Axum backend applications.
//!
//! ## Overview
//!
//! - **[`idempotency`]**: Redis-backed API request deduplication, atomic key reservation, nil-safe cache reads, and response caching.
//! - **[`pagination`]**: Base64 URL-safe UUID cursor pagination helpers and Axum query parameter extractors.
//! - **[`webhook`]**: Constant-time HMAC-SHA256 signature verification supporting both direct digest webhooks (GitHub, Shopify) and timestamped replay-safe signatures (Stripe, Svix) with typed Axum extractor ([`VerifiedWebhook`]).
//! - **[`worker`]**: Periodic background worker task loops with panic isolation, cancellation signals, and graceful shutdown waiting for active tasks to cleanly finish.
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use scyph_utils::pagination::{PageParams, Cursor};
//! use scyph_utils::webhook::{verify_webhook_header, verify_raw_webhook, VerifiedWebhook};
//! use scyph_utils::idempotency::{IdempotencyStore, IdempotencyConfig};
//! use scyph_utils::worker::spawn_worker_cancel;
//! ```

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
pub use webhook::{
    VerifiedWebhook, WebhookConfig, WebhookError, verify_raw_webhook, verify_webhook,
    verify_webhook_header,
};

#[doc(inline)]
pub use worker::{WorkerError, WorkerHandle, spawn_worker, spawn_worker_cancel};
