//! # Scyph DB
//!
//! `scyph-db` provides database connection pooling, transaction management,
//! migration runner utilities, and dynamic SQL query building extensions for PostgreSQL using `sqlx`.
//!
//! ## Overview
//!
//! - **Connection Pooling ([`build_pool`])**: Tunable PostgreSQL connection pool builder using environment variables.
//! - **Transactions ([`begin`], [`commit`], [`rollback`])**: Ergonomic transaction lifecycle management.
//! - **Migrations & Seeds ([`run_migrations`], [`run_seeds`])**: Automated SQL migration runner and database seeding helpers.
//! - **Dynamic Queries ([`query`])**: Type-safe `QueryBuilder` extension traits for multi-field ILIKE search, column filtering, ORDER BY sorting, and pagination.
//!
//! ## Feature Flags
//!
//! - **`health`**: Enables [`DbHealthExt`] integration with `scyph-health` registries for Kubernetes probes.
//! - **`query`**: Enables dynamic SQL query composition traits ([`ApplySearch`], [`ApplyFiltering`], [`ApplySorting`], [`ApplyPagination`], [`ApplyRequestParams`]).

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

#[cfg(feature = "query")]
/// QueryBuilder extensions for search, filtering, sorting, and pagination.
pub mod query;

/// Database error type definitions.
pub mod error;

/// Internal utility helpers for filesystem and environment resolution.
pub(crate) mod util;

pub use error::DbError;
pub use migrate::{run_migrations, run_migrations_from, run_seeds, run_seeds_from};
pub use pool::{DbConfig, build_pool, build_pool_with_config};
pub use transaction::{begin, commit, rollback};

#[cfg(feature = "health")]
pub use health::DbHealthExt;

#[cfg(feature = "query")]
pub use query::{
    ApplyFiltering, ApplyPagination, ApplyRequestParams, ApplySearch, ApplySorting,
    escape_like_pattern,
};
