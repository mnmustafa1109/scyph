//! # Scyph DB
//!
//! `scyph-db` provides database connection pooling, transaction management,
//! and migration runner utilities for PostgreSQL using `sqlx`.

#![warn(missing_docs)]

/// Database migration runner and seed data execution helpers.
pub mod migrate;

/// Connection pool builder for PostgreSQL.
pub mod pool;

/// Database transaction lifecycle wrappers.
pub mod transaction;

pub use migrate::{run_migrationes, run_seeds};
pub use pool::build_pool;
pub use transaction::{begin, commit, rollback};
