//! ORDER BY sorting clause composition.

use scyph_extractors::query::{SortOrder, SortParams};
use sqlx::{Postgres, QueryBuilder};

use super::constants::{ASCENDING, DESCENDING, ORDER_BY};

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
