//! LIMIT and OFFSET pagination clause composition.
//!
//! This module provides the [`ApplyPagination`] extension trait, which appends `LIMIT` and
//! `OFFSET` clauses to a [`QueryBuilder<Postgres>`] based on 1-indexed page numbers.
//!
//! ## Offset Calculation
//!
//! Given `page` (1-indexed) and `limit` (items per page), the offset is calculated as:
//!
//! ```text
//! offset = (page - 1) * limit
//! ```
//!
//! Examples:
//! - `page=1, limit=20` → `LIMIT 20 OFFSET 0` (first page)
//! - `page=2, limit=20` → `LIMIT 20 OFFSET 20` (second page)
//! - `page=3, limit=20` → `LIMIT 20 OFFSET 40` (third page)
//!
//! ## Overflow Protection
//!
//! - `limit` is clamped to a minimum of `1` via `.max(1)` to prevent `LIMIT 0` queries.
//! - `page` uses [`u64::saturating_sub`] before multiplying to prevent underflow on `page=0`.
//! - The multiplication uses [`i64::saturating_mul`] to prevent overflow on extremely large
//!   page numbers.
//! - Both `LIMIT` and `OFFSET` values are bound as `i64` parameters via `push_bind`.
//!
//! ## Usage
//!
//! Pagination must come after all `WHERE`, `ORDER BY` clauses. When using
//! [`crate::ApplyRequestParams::apply_request_params`], ordering is enforced automatically.

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
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1 ORDER BY id ASC");
///
/// // Page 3 with 10 items per page → LIMIT 10 OFFSET 20
/// let pagination = PaginationParams { page: 3, limit: 10 };
/// qb.apply_pagination(&pagination);
///
/// // Page 1 with 20 items per page → LIMIT 20 OFFSET 0
/// let mut qb2 = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1 ORDER BY id ASC");
/// qb2.apply_pagination(&PaginationParams { page: 1, limit: 20 });
/// ```
pub trait ApplyPagination {
    /// Appends `LIMIT $limit OFFSET $offset` calculated from 1-indexed page numbers.
    ///
    /// The offset is derived as `(page - 1) * limit`. Both values are bound as `i64`
    /// parameters and are protected against underflow and overflow via saturating arithmetic.
    ///
    /// # Arguments
    ///
    /// * `pagination` - Reference to incoming [`PaginationParams`] containing the 1-indexed
    ///   page number and the number of items per page.
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
