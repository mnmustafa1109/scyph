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
//! ## Crate Architecture
//!
//! Each module is independently usable. Import only what you need:
//!
//! ```rust,ignore
//! // Webhook verification
//! use scyph_utils::webhook::{VerifiedWebhook, WebhookConfig};
//!
//! // Cursor pagination
//! use scyph_utils::pagination::{Cursor, PageParams};
//!
//! // Idempotent request handling
//! use scyph_utils::idempotency::{IdempotencyStore, IdempotencyConfig};
//!
//! // Background periodic workers
//! use scyph_utils::worker::{spawn_worker_cancel, WorkerHandle};
//! ```
//!
//! ## Quick Examples
//!
//! ### Cursor pagination
//!
//! ```rust,ignore
//! use scyph_utils::pagination::{Cursor, PageParams};
//! use axum::extract::Query;
//! use uuid::Uuid;
//!
//! async fn list_users(Query(params): Query<PageParams>) -> impl axum::response::IntoResponse {
//!     let after_id: Option<Uuid> = params.cursor_id().unwrap_or(None);
//!     let limit = params.limit(); // clamped to [1, 100]
//!
//!     // Query DB: SELECT * FROM users WHERE id > $1 ORDER BY id LIMIT $2
//!     let mut rows = fetch_users(after_id, limit + 1).await;
//!     let page = Cursor::build_page(&mut rows, limit, |u| u.id);
//!     axum::Json(page)
//! }
//! ```
//!
//! ### Idempotent payment handler
//!
//! ```rust,ignore
//! use scyph_utils::idempotency::IdempotencyStore;
//! use axum::extract::Extension;
//!
//! async fn charge_handler(
//!     Extension(idem): Extension<IdempotencyStore>,
//!     idempotency_key: String,
//! ) -> axum::Json<PaymentResult> {
//!     let result = idem.execute(&idempotency_key, || async {
//!         process_payment().await
//!     }).await?;
//!     axum::Json(result)
//! }
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
