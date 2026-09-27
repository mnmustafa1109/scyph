//! Composite request parameter clause composition.

use scyph_extractors::query::RequestParams;
use sqlx::{Postgres, QueryBuilder};
use strum::IntoEnumIterator;

use super::{
    filter::ApplyFiltering, pagination::ApplyPagination, search::ApplySearch, sort::ApplySorting,
};

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
    /// Applies search and filtering WHERE conditions to the query builder.
    ///
    /// Omits ORDER BY and LIMIT/OFFSET clauses, making it ideal for `SELECT COUNT(*)` queries
    /// or for composing query conditions prior to pagination and sorting.
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
    /// # Arguments
    ///
    /// * `params` - Reference to incoming composite [`RequestParams`].
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
