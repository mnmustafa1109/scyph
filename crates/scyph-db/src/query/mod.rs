//! Dynamic SQL query composition extensions for `sqlx::QueryBuilder`.
//!
//! Provides type-safe extension traits that append SQL clauses to a
//! [`sqlx::QueryBuilder<Postgres>`] in a composable, injection-safe way. All traits operate
//! on query builders that already have a `WHERE 1=1` or equivalent base clause, allowing
//! each extension to unconditionally prepend `AND` without conditional branching at the
//! call site.
//!
//! ## Traits at a Glance
//!
//! | Trait | SQL Produced | Input Type |
//! |-------|-------------|------------|
//! | [`ApplySearch`] | `AND (col1 ILIKE $1 OR col2 ILIKE $1 ...)` | [`SearchParams`](scyph_extractors::query::SearchParams) |
//! | [`ApplyFiltering`] | `AND col::text = $n` | [`FilterParams`](scyph_extractors::query::FilterParams) |
//! | [`ApplySorting`] | `ORDER BY col ASC\|DESC` | [`SortParams`](scyph_extractors::query::SortParams) |
//! | [`ApplyPagination`] | `LIMIT $n OFFSET $m` | [`PaginationParams`](scyph_extractors::query::PaginationParams) |
//! | [`ApplyRequestParams`] | All of the above (conditions + sort + page) | [`RequestParams`](scyph_extractors::query::RequestParams) |
//!
//! ## Usage Pattern
//!
//! Each trait is implemented directly on `QueryBuilder<Postgres>`. The recommended pattern
//! is to start with a base query that includes `WHERE 1=1` so that every subsequent
//! `AND ...` clause is always syntactically valid regardless of whether preceding clauses
//! were added:
//!
//! ```rust,ignore
//! use scyph_db::{ApplySearch, ApplyFiltering, ApplySorting, ApplyPagination};
//! use scyph_extractors::query::{FilterParams, PaginationParams, SearchParams, SortOrder, SortParams};
//! use sqlx::{Postgres, QueryBuilder};
//! use strum_macros::EnumIter;
//!
//! #[derive(Copy, Clone, EnumIter)]
//! enum SearchCol { Name, Email }
//!
//! impl From<SearchCol> for &'static str {
//!     fn from(c: SearchCol) -> Self {
//!         match c {
//!             SearchCol::Name  => "users.name",
//!             SearchCol::Email => "users.email",
//!         }
//!     }
//! }
//!
//! #[derive(Copy, Clone)]
//! enum FilterCol { Status }
//!
//! impl From<FilterCol> for &'static str {
//!     fn from(_: FilterCol) -> Self { "users.status" }
//! }
//!
//! #[derive(Copy, Clone)]
//! enum SortCol { CreatedAt }
//!
//! impl From<SortCol> for &'static str {
//!     fn from(_: SortCol) -> Self { "users.created_at" }
//! }
//!
//! let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
//!
//! qb.apply_search::<SearchCol>(&SearchParams { q: Some("alice".into()) });
//! qb.apply_filtering(&FilterParams { filter_by: Some(FilterCol::Status), filter_value: Some("active".into()) });
//! qb.apply_sorting(&SortParams { sort_by: Some(SortCol::CreatedAt), sort_order: Some(SortOrder::Desc) });
//! qb.apply_pagination(&PaginationParams { page: 2, limit: 25 });
//!
//! // Produces roughly:
//! // SELECT * FROM users WHERE 1=1
//! //   AND (users.name ILIKE $1 OR users.email ILIKE $1)
//! //   AND users.status::text = $2
//! //   ORDER BY users.created_at DESC
//! //   LIMIT $3 OFFSET $4
//! let query = qb.build();
//! ```
//!
//! ## Security
//!
//! All user-supplied values are bound with `push_bind()`, which generates parameterized
//! query placeholders (`$1`, `$2`, …) rather than interpolating values directly into the
//! SQL string. Special LIKE pattern characters (`%`, `_`, `\`) in search strings are
//! automatically escaped by [`escape_like_pattern`] before binding.

pub mod constants;
pub mod filter;
pub mod pagination;
pub mod params;
pub mod search;
pub mod sort;

pub use constants::*;
pub use filter::*;
pub use pagination::*;
pub use params::*;
pub use search::*;
pub use sort::*;
