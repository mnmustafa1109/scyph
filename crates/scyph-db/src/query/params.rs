//! Composite request parameter clause composition.
//!
//! This module provides [`ApplyRequestParams`], a convenience extension trait that applies
//! the full set of query modifiers (search, filter, sort, pagination) to a
//! [`QueryBuilder<Postgres>`] in a single call, using a composite [`RequestParams`] value
//! that bundles all four parameter types together.
//!
//! ## When to Use
//!
//! - **[`apply_request_params`](ApplyRequestParams::apply_request_params)** — use for the
//!   main data-fetching query that returns paginated rows. Applies all four modifiers in the
//!   correct SQL clause order: search + filter → sort → pagination.
//!
//! - **[`apply_conditions`](ApplyRequestParams::apply_conditions)** — use for the companion
//!   `SELECT COUNT(*)` query that counts total matching rows. Applies only search + filter
//!   (no `ORDER BY` or `LIMIT`/`OFFSET`) because ordering and pagination are irrelevant for
//!   counting rows.
//!
//! ## Type Parameters
//!
//! Both methods require three type parameters at the call site:
//!
//! - `E` — the search column enum (must implement [`strum::IntoEnumIterator`], `Into<&'static str>`, `Copy`).
//! - `S` — the sort column enum (must implement `Into<&'static str>`, `Copy`).
//! - `F` — the filter column enum (must implement `Into<&'static str>`, `Copy`).
//!
//! ## Full Example
//!
//! ```rust,ignore
//! use scyph_db::ApplyRequestParams;
//! use scyph_extractors::query::{FilterParams, PaginationParams, RequestParams, SearchParams, SortParams};
//! use sqlx::{Postgres, QueryBuilder};
//! use strum_macros::EnumIter;
//!
//! #[derive(Copy, Clone, EnumIter)]
//! enum SearchCol { Name, Email }
//! impl From<SearchCol> for &'static str {
//!     fn from(c: SearchCol) -> Self {
//!         match c { SearchCol::Name => "u.name", SearchCol::Email => "u.email" }
//!     }
//! }
//!
//! #[derive(Copy, Clone)]
//! enum SortCol { CreatedAt }
//! impl From<SortCol> for &'static str { fn from(_: SortCol) -> Self { "u.created_at" } }
//!
//! #[derive(Copy, Clone)]
//! enum FilterCol { Role }
//! impl From<FilterCol> for &'static str { fn from(_: FilterCol) -> Self { "u.role" } }
//!
//! async fn list_users(pool: &sqlx::PgPool, params: RequestParams<SortCol, FilterCol>) {
//!     // Count query — no sort or pagination
//!     let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM users u WHERE 1=1");
//!     count_qb.apply_conditions::<SearchCol>(&params);
//!
//!     // Data query — full sort + pagination
//!     let mut data_qb = QueryBuilder::<Postgres>::new("SELECT * FROM users u WHERE 1=1");
//!     data_qb.apply_request_params::<SearchCol>(&params);
//! }
//! ```

use scyph_extractors::query::RequestParams;
use sqlx::{Postgres, QueryBuilder};
use strum::IntoEnumIterator;

use super::{
    filter::ApplyFiltering, pagination::ApplyPagination, search::ApplySearch, sort::ApplySorting,
};

/// Extension trait for appending full composite [`RequestParams`] (search, filter, sort, pagination) in a single call.
///
/// Implemented on `QueryBuilder<Postgres>`. The generic parameters `S` and `F` represent
/// the sort and filter column enums respectively. The search column enum `E` is specified
/// at the call site as a turbofish type parameter.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplyRequestParams;
/// use scyph_extractors::query::RequestParams;
/// use sqlx::{Postgres, QueryBuilder};
///
/// // E = SearchCol, S = SortCol, F = FilterCol (caller-defined enums)
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users u WHERE 1=1");
/// qb.apply_request_params::<SearchCol>(&params);
/// // → AND (u.name ILIKE $1 ...) AND u.role::text = $2 ORDER BY u.created_at DESC LIMIT $3 OFFSET $4
///
/// // For COUNT(*) companion query — omit sort and pagination
/// let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM users u WHERE 1=1");
/// count_qb.apply_conditions::<SearchCol>(&params);
/// // → AND (u.name ILIKE $1 ...) AND u.role::text = $2
/// ```
pub trait ApplyRequestParams<S, F> {
    /// Applies search and filtering `WHERE` conditions to the query builder.
    ///
    /// Calls [`apply_search`](crate::ApplySearch::apply_search) then
    /// [`apply_filtering`](crate::ApplyFiltering::apply_filtering) in sequence. Does **not**
    /// append `ORDER BY` or `LIMIT`/`OFFSET`, making it ideal for `SELECT COUNT(*)`
    /// companion queries where sorting and pagination are unnecessary.
    ///
    /// # Arguments
    ///
    /// * `params` - Reference to incoming composite [`RequestParams`].
    fn apply_conditions<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>;

    /// Applies search, filtering, sorting, and pagination in sequence to the query builder.
    ///
    /// Calls the four individual trait methods in the correct SQL clause order:
    /// 1. [`apply_search`](crate::ApplySearch::apply_search) (WHERE conditions)
    /// 2. [`apply_filtering`](crate::ApplyFiltering::apply_filtering) (WHERE conditions)
    /// 3. [`apply_sorting`](crate::ApplySorting::apply_sorting) (ORDER BY)
    /// 4. [`apply_pagination`](crate::ApplyPagination::apply_pagination) (LIMIT / OFFSET)
    ///
    /// # Arguments
    ///
    /// * `params` - Reference to incoming composite [`RequestParams`] containing all four
    ///   parameter groups.
    fn apply_request_params<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>;
}

impl<S, F> ApplyRequestParams<S, F> for QueryBuilder<Postgres> {
    fn apply_conditions<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>,
    {
        self.apply_search::<E>(&params.search);
        self.apply_filtering(&params.filter);
    }

    fn apply_request_params<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>,
    {
        self.apply_conditions::<E>(params);
        self.apply_sorting(&params.sort);
        self.apply_pagination(&params.pagination);
    }
}
