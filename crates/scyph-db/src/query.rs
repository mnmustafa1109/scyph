//! Dynamic SQL query composition extensions for `sqlx::QueryBuilder`.
//!
//! Provides type-safe traits to append full-text search ILIKE filters, column equality constraints,
//! ORDER BY sorts, and LIMIT / OFFSET pagination to PostgreSQL query builders.

use scyph_extractors::query::{
    FilterParams, PaginationParams, RequestParams, SearchParams, SortOrder, SortParams,
};
use sqlx::{Postgres, QueryBuilder};
use strum::IntoEnumIterator;

/// SQL constant for AND prefix.
const AND_PREFIX: &str = " AND ";

/// SQL constant opening an AND search group.
const AND_OPEN_GROUP: &str = " AND (";

/// SQL constant for OR delimiter between search columns.
const OR_DELIMITER: &str = " OR ";

/// SQL constant closing a parenthesis group.
const CLOSE_GROUP: &str = ")";

/// SQL constant for case-insensitive LIKE comparison.
const ILIKE: &str = " ILIKE ";

/// SQL constant for ORDER BY sorting.
const ORDER_BY: &str = " ORDER BY ";

/// SQL constant for ascending sort order.
const ASCENDING: &str = " ASC";

/// SQL constant for descending sort order.
const DESCENDING: &str = " DESC";

/// SQL constant for LIMIT clause.
const LIMIT: &str = " LIMIT ";

/// SQL constant for OFFSET clause.
const OFFSET: &str = " OFFSET ";

/// SQL constant for text-cast equality comparison (`::text = `).
const CAST_TEXT_EQUAL: &str = "::text = ";

/// SQL wildcard symbol for partial pattern matching.
const WILDCARD: char = '%';

/// Helper function to escape special SQL LIKE pattern wildcards (`%`, `_`, `\`).
///
/// Ensures user-supplied search strings match literal text rather than behaving as wildcards.
#[inline]
pub fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Extension trait for appending full-text search ILIKE filters to a [`QueryBuilder`].
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplySearch;
/// use scyph_extractors::query::SearchParams;
/// use sqlx::{Postgres, QueryBuilder};
/// use strum_macros::EnumIter;
///
/// #[derive(Copy, Clone, EnumIter)]
/// enum UserSearch { Name, Email }
/// impl From<UserSearch> for &'static str {
///     fn from(s: UserSearch) -> Self {
///         match s { UserSearch::Name => "users.name", UserSearch::Email => "users.email" }
///     }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let search = SearchParams { q: Some("alice".into()) };
/// qb.apply_search::<UserSearch>(&search);
/// ```
pub trait ApplySearch {
    /// Appends ILIKE search clauses across all fields defined by enum `E`.
    ///
    /// If `search.q` contains non-whitespace text, appends `AND (col1 ILIKE $1 OR col2 ILIKE $1 ...)`.
    ///
    /// # Arguments
    ///
    /// * `search` - Reference to incoming [`SearchParams`].
    fn apply_search<E>(&mut self, search: &SearchParams)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy;
}

/// Extension trait for appending single-column filter conditions to a [`QueryBuilder`].
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplyFiltering;
/// use scyph_extractors::query::FilterParams;
/// use sqlx::{Postgres, QueryBuilder};
///
/// #[derive(Copy, Clone)]
/// enum StatusFilter { Active }
/// impl From<StatusFilter> for &'static str {
///     fn from(_: StatusFilter) -> Self { "users.status" }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let filters = FilterParams { filter_by: Some(StatusFilter::Active), filter_value: Some("true".into()) };
/// qb.apply_filtering(&filters);
/// ```
pub trait ApplyFiltering<F> {
    /// Appends `AND column::text = $value` when both filter field and value are present.
    ///
    /// # Arguments
    ///
    /// * `filters` - Reference to incoming [`FilterParams`].
    fn apply_filtering(&mut self, filters: &FilterParams<F>)
    where
        F: Copy + Into<&'static str>;
}

/// Extension trait for appending ORDER BY sorting clauses to a [`QueryBuilder`].
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplySorting;
/// use scyph_extractors::query::{SortParams, SortOrder};
/// use sqlx::{Postgres, QueryBuilder};
///
/// #[derive(Copy, Clone)]
/// enum UserSort { CreatedAt }
/// impl From<UserSort> for &'static str {
///     fn from(_: UserSort) -> Self { "users.created_at" }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let sort = SortParams { sort_by: Some(UserSort::CreatedAt), sort_order: Some(SortOrder::Desc) };
/// qb.apply_sorting(&sort);
/// ```
pub trait ApplySorting<S> {
    /// Appends `ORDER BY column ASC` or `ORDER BY column DESC` to the query builder.
    ///
    /// Defaults to `ASC` if sort order is omitted or specified as ascending.
    ///
    /// # Arguments
    ///
    /// * `sort` - Reference to incoming [`SortParams`].
    fn apply_sorting(&mut self, sort: &SortParams<S>)
    where
        S: Copy + Into<&'static str>;
}

/// Extension trait for appending LIMIT and OFFSET pagination clauses to a [`QueryBuilder`].
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplyPagination;
/// use scyph_extractors::query::PaginationParams;
/// use sqlx::{Postgres, QueryBuilder};
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let pagination = PaginationParams { page: 2, limit: 20 };
/// qb.apply_pagination(&pagination);
/// ```
pub trait ApplyPagination {
    /// Appends `LIMIT $limit OFFSET $offset` calculated from 1-indexed page numbers.
    ///
    /// # Arguments
    ///
    /// * `pagination` - Reference to incoming [`PaginationParams`].
    fn apply_pagination(&mut self, pagination: &PaginationParams);
}

/// Extension trait for appending full composite [`RequestParams`] (search, filter, sort, pagination) in a single call.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplyRequestParams;
/// use scyph_extractors::query::RequestParams;
/// use sqlx::{Postgres, QueryBuilder};
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// qb.apply_request_params::<UserSearch, UserSort, UserFilter>(&params);
/// ```
pub trait ApplyRequestParams<S, F> {
    /// Applies search, filtering, sorting, and pagination in sequence to the query builder.
    ///
    /// # Arguments
    ///
    /// * `params` - Reference to incoming composite [`RequestParams`].
    fn apply_request_params<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>;
}

impl ApplySearch for QueryBuilder<Postgres> {
    fn apply_search<E>(&mut self, search: &SearchParams)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
    {
        if let Some(q) = &search.q {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                let mut fields = E::iter().peekable();
                if fields.peek().is_none() {
                    return;
                }
                let escaped = escape_like_pattern(trimmed);
                self.push(AND_OPEN_GROUP);
                let mut first = true;
                for field in fields {
                    if !first {
                        self.push(OR_DELIMITER);
                    }
                    first = false;
                    let field_str: &str = field.into();
                    self.push(field_str);
                    self.push(ILIKE);
                    self.push_bind(format!("{WILDCARD}{escaped}{WILDCARD}"));
                }
                self.push(CLOSE_GROUP);
            }
        }
    }
}

impl<F> ApplyFiltering<F> for QueryBuilder<Postgres>
where
    F: Copy + Into<&'static str>,
{
    fn apply_filtering(&mut self, filters: &FilterParams<F>) {
        let field_name = filters.filter_by.map(|f| f.into());
        if let (Some(field), Some(value)) = (field_name, &filters.filter_value) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                self.push(AND_PREFIX);
                self.push(field);
                self.push(CAST_TEXT_EQUAL);
                self.push_bind(trimmed.to_string());
            }
        }
    }
}

impl<S> ApplySorting<S> for QueryBuilder<Postgres>
where
    S: Copy + Into<&'static str>,
{
    fn apply_sorting(&mut self, sort: &SortParams<S>) {
        if let Some(field) = sort.sort_by {
            let order = match sort.sort_order {
                Some(SortOrder::Desc) => DESCENDING,
                _ => ASCENDING,
            };
            self.push(ORDER_BY);
            self.push(field.into());
            self.push(order);
        }
    }
}

impl ApplyPagination for QueryBuilder<Postgres> {
    fn apply_pagination(&mut self, pagination: &PaginationParams) {
        let limit = pagination.limit.max(1) as i64;
        let offset = (pagination.page.saturating_sub(1) as i64).saturating_mul(limit);
        self.push(LIMIT);
        self.push_bind(limit);
        self.push(OFFSET);
        self.push_bind(offset);
    }
}

impl<S, F> ApplyRequestParams<S, F> for QueryBuilder<Postgres> {
    fn apply_request_params<E>(&mut self, params: &RequestParams<S, F>)
    where
        E: IntoEnumIterator + Into<&'static str> + Copy,
        S: Copy + Into<&'static str>,
        F: Copy + Into<&'static str>,
    {
        self.apply_search::<E>(&params.search);
        self.apply_filtering(&params.filter);
        self.apply_sorting(&params.sort);
        self.apply_pagination(&params.pagination);
    }
}
