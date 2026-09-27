//! Redis-backed API idempotency store for request deduplication and response caching.

pub mod config;
pub mod error;
pub mod store;

pub use config::IdempotencyConfig;
pub use error::IdempotencyError;
pub use store::{IdempotencyCheck, IdempotencyStore};
