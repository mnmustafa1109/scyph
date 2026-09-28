//! ORDER BY sorting clause composition.
//!
//! This module provides the [`ApplySorting`] extension trait, which appends an `ORDER BY`
//! clause to a [`QueryBuilder<Postgres>`].
//!
//! ## How It Works
//!
//! When a sort column variant is present in [`SortParams`], `apply_sorting` appends
//! `ORDER BY col ASC` or `ORDER BY col DESC` where:
//!
//! - The column name comes from calling `.into()` on the `sort_by` variant.
//! - The direction defaults to `ASC` if `sort_order` is `None` or `Some(SortOrder::Asc)`.
//! - If `sort_by` is `None`, no clause is appended and the natural PostgreSQL result order
//!   (typically insertion order for heap tables) is preserved.
//!
//! ## Ordering Relative to Other Clauses
//!
//! `ORDER BY` must come after all `WHERE` conditions and before `LIMIT`/`OFFSET`. When using
//! [`crate::ApplyRequestParams::apply_request_params`], the call order is enforced automatically.
//! When composing manually, always call `apply_sorting` before `apply_pagination`.
//!
//! ## Note on Injection Safety
//!
//! Column names and direction keywords (`ASC`/`DESC`) are pushed via `push()` rather than
//! `push_bind()` because PostgreSQL does not support parameterized identifiers or keywords.
//! The `sort_by` column must be a `&'static str` defined at compile time by the caller's
//! enum — never a user-supplied runtime string — which prevents SQL injection.

use scyph_extractors::query::{SortOrder, SortParams};
use sqlx::{Postgres, QueryBuilder};

use super::constants::{ASCENDING, DESCENDING, ORDER_BY};

/// Extension trait for appending ORDER BY sorting clauses to a [`QueryBuilder`].
///
/// The generic parameter `S` is the caller-defined enum whose variants represent the
/// sortable columns. Each variant must implement `Into<&'static str>` and `Copy`.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplySorting;
/// use scyph_extractors::query::{SortParams, SortOrder};
/// use sqlx::{Postgres, QueryBuilder};
///
/// #[derive(Copy, Clone)]
/// enum UserSort { CreatedAt, Name }
///
/// impl From<UserSort> for &'static str {
///     fn from(s: UserSort) -> Self {
///         match s {
///             UserSort::CreatedAt => "users.created_at",
///             UserSort::Name      => "users.name",
///         }
///     }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
///
/// // Sort by creation date descending
/// let sort = SortParams { sort_by: Some(UserSort::CreatedAt), sort_order: Some(SortOrder::Desc) };
/// qb.apply_sorting(&sort);
/// // Appended: ORDER BY users.created_at DESC
///
/// // With no sort order specified, defaults to ASC
/// let mut qb2 = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let sort2 = SortParams { sort_by: Some(UserSort::Name), sort_order: None };
/// qb2.apply_sorting(&sort2);
/// // Appended: ORDER BY users.name ASC
/// ```
pub trait ApplySorting<S> {
    /// Appends `ORDER BY column ASC` or `ORDER BY column DESC` to the query builder.
    ///
    /// If `sort.sort_by` is `None`, the query builder is left unmodified (natural result
    /// order is preserved). Defaults to `ASC` when `sort.sort_order` is `None`.
    ///
    /// # Arguments
    ///
    /// * `sort` - Reference to incoming [`SortParams`] containing the optional column selector
    ///   and sort direction.
    fn apply_sorting(&mut self, sort: &SortParams<S>)
    where
        S: Copy + Into<&'static str>;
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
