//! Redis-backed API idempotency store for request deduplication and response caching.
//!
//! Idempotency ensures that retrying a failed or timed-out request produces the same
//! outcome as the original execution, without duplicating side effects like payments, emails,
//! or database inserts.
//!
//! ## How It Works
//!
//! The client sends a unique idempotency key (e.g., a client-generated UUID) in a request
//! header. The server:
//!
//! 1. Attempts to **atomically reserve** the key in Redis using `SET key IN_PROGRESS NX EX ttl`.
//! 2. If the key was **new**: executes the handler, caches the response JSON, and returns it.
//! 3. If the key was already **completed**: returns the cached JSON response immediately (idempotent replay).
//! 4. If a concurrent request with the same key is **in-progress**: returns HTTP 409 Conflict.
//!
//! ## Quickstart
//!
//! ```rust,no_run
//! use scyph_utils::idempotency::{IdempotencyStore, IdempotencyConfig, IdempotencyError};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct PaymentResult { transaction_id: String, amount: u64 }
//!
//! async fn charge_card(key: &str, amount: u64) -> Result<PaymentResult, Box<dyn std::error::Error>> {
//!     let store = IdempotencyStore::from_env()?;
//!     let result = store.execute(key, || async move {
//!         // This block runs at most once per unique key
//!         Ok::<PaymentResult, IdempotencyError>(PaymentResult {
//!             transaction_id: "tx_abc123".into(),
//!             amount,
//!         })
//!     }).await?;
//!     Ok(result)
//! }
//! ```
//!
//! ## Redis Key Schema
//!
//! Keys are stored as `{prefix}:{idempotency_key}`. During execution the value is
//! `"IN_PROGRESS"`. After completion it holds the serialized JSON response body.
//! Keys expire after the configured TTL (default: 24 hours).

pub mod config;
pub mod error;
pub mod store;

pub use config::IdempotencyConfig;
pub use error::IdempotencyError;
pub use store::{IdempotencyCheck, IdempotencyStore};
