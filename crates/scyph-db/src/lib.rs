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

#[cfg(feature = "health")]
/// Extension traits for automated PostgreSQL pool health checks.
pub mod health;

/// Database error type definitions.
pub mod error;

pub use error::DbError;
pub use migrate::{run_migrations, run_migrations_from, run_seeds, run_seeds_from};
pub use pool::build_pool;
pub use transaction::{begin, commit, rollback};

#[cfg(feature = "health")]
pub use health::DbHealthExt;
