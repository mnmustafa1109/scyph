//! # Scyph DB
//!
//! `scyph-db` provides database connection pooling, transaction management,
//! migration runner utilities, and dynamic SQL query building extensions for PostgreSQL using `sqlx`.
//!
//! ## Overview
//!
//! - **Connection Pooling ([`build_pool`])**: Tunable PostgreSQL connection pool builder driven by
//!   environment variables (`DATABASE_URL`, `MAX_CONNECTIONS`, `MIN_CONNECTIONS`).
//! - **Transactions ([`begin`], [`commit`], [`rollback`])**: Ergonomic transaction lifecycle
//!   management wrapping `sqlx::Transaction` with typed error propagation.
//! - **Migrations & Seeds ([`run_migrations`], [`run_seeds`])**: Automated cascading SQL migration
//!   runner and database seeding helpers that resolve per-environment subdirectories automatically.
//! - **Dynamic Queries ([`query`])**: Type-safe `QueryBuilder` extension traits for multi-field
//!   ILIKE search, column equality filtering, ORDER BY sorting, and LIMIT/OFFSET pagination.
//!
//! ## Feature Flags
//!
//! Features are **opt-in** and are activated by adding them to your `Cargo.toml` dependency entry:
//!
//! ```toml
//! [dependencies]
//! scyph-db = { version = "0.1", features = ["health", "query"] }
//! ```
//!
//! ### `health`
//!
//! Enables [`DbHealthExt`], which adds `check_health` and `check_health_named` methods directly
//! on `sqlx::PgPool`. These methods execute a `SELECT 1` ping query and report the result to
//! a [`scyph_health::HealthRegistry`], making it easy to integrate PostgreSQL liveness into
//! Kubernetes `/readyz` probes.
//!
//! Requires the `scyph-health` crate to be present as a dependency.
//!
//! ```rust,ignore
//! // With feature "health" enabled:
//! use scyph_db::{build_pool, DbHealthExt};
//! use scyph_health::HealthRegistry;
//!
//! async fn monitor(registry: &HealthRegistry) {
//!     let pool = build_pool().await.expect("Pool");
//!     pool.check_health(registry).await;          // reports to "database"
//!     pool.check_health_named(registry, "replica", false).await;
//! }
//! ```
//!
//! ### `query`
//!
//! Enables the full suite of dynamic SQL query composition traits that extend
//! `sqlx::QueryBuilder<Postgres>`:
//!
//! | Trait | Purpose |
//! |-------|---------|
//! | [`ApplySearch`] | Appends multi-column `ILIKE` full-text search clauses |
//! | [`ApplyFiltering`] | Appends single-column `::text =` equality filter clauses |
//! | [`ApplySorting`] | Appends `ORDER BY col ASC/DESC` sorting clauses |
//! | [`ApplyPagination`] | Appends `LIMIT $n OFFSET $m` pagination clauses |
//! | [`ApplyRequestParams`] | Applies all of the above in a single call from [`scyph_extractors::query::RequestParams`] |
//!
//! Also re-exports [`escape_like_pattern`] for sanitizing user-supplied search strings.
//!
//! ```rust,ignore
//! // With feature "query" enabled:
//! use scyph_db::{ApplySearch, ApplyPagination, escape_like_pattern};
//! use scyph_extractors::query::{PaginationParams, SearchParams};
//! use sqlx::{Postgres, QueryBuilder};
//! use strum_macros::EnumIter;
//!
//! #[derive(Copy, Clone, EnumIter)]
//! enum UserSearch { Name, Email }
//!
//! impl From<UserSearch> for &'static str {
//!     fn from(s: UserSearch) -> Self {
//!         match s {
//!             UserSearch::Name  => "users.name",
//!             UserSearch::Email => "users.email",
//!         }
//!     }
//! }
//!
//! let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
//! qb.apply_search::<UserSearch>(&SearchParams { q: Some("alice".into()) });
//! qb.apply_pagination(&PaginationParams { page: 1, limit: 20 });
//! ```

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
pub use pool::build_pool;
pub use transaction::{begin, commit, rollback};

#[cfg(feature = "health")]
pub use health::DbHealthExt;

#[cfg(feature = "query")]
pub use query::{
    ApplyFiltering, ApplyPagination, ApplyRequestParams, ApplySearch, ApplySorting,
    escape_like_pattern,
};
