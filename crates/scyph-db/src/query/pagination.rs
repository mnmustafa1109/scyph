//! LIMIT and OFFSET pagination clause composition.

use scyph_extractors::query::PaginationParams;
use sqlx::{Postgres, QueryBuilder};

use super::constants::{LIMIT, OFFSET};

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
