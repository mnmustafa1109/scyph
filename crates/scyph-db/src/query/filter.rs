//! Single-column equality filter composition.
//!
//! This module provides the [`ApplyFiltering`] extension trait, which appends a typed
//! single-column equality filter clause to a [`QueryBuilder<Postgres>`].
//!
//! ## How It Works
//!
//! When both `filter_by` (column) and `filter_value` (value) are present in [`FilterParams`],
//! `apply_filtering` appends `AND col::text = $n` where:
//!
//! - The column name is derived by calling `.into()` on the `filter_by` variant, which must
//!   implement `Into<&'static str>`.
//! - The value is cast to `text` in PostgreSQL using `::text`, enabling filtering on columns
//!   of any type (UUID, integer, enum, etc.) without requiring exact type matching on the
//!   client side.
//! - The value is bound via `push_bind` — never interpolated directly — preventing SQL injection.
//! - Leading and trailing whitespace is stripped from the filter value before binding. If the
//!   trimmed value is empty, no clause is appended.
//!
//! ## Typical Use
//!
//! Filtering is typically applied after search conditions and before sorting/pagination:
//!
//! ```rust,ignore
//! use scyph_db::ApplyFiltering;
//! use scyph_extractors::query::FilterParams;
//! use sqlx::{Postgres, QueryBuilder};
//!
//! #[derive(Copy, Clone)]
//! enum UserFilter { Role }
//!
//! impl From<UserFilter> for &'static str {
//!     fn from(_: UserFilter) -> Self { "users.role" }
//! }
//!
//! let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
//! let filter = FilterParams {
//!     filter_by: Some(UserFilter::Role),
//!     filter_value: Some("admin".into()),
//! };
//! qb.apply_filtering(&filter);
//! // Appended: AND users.role::text = $1
//! // Bound:    $1 = "admin"
//! ```

use scyph_extractors::query::FilterParams;
use sqlx::{Postgres, QueryBuilder};

use super::constants::{AND_PREFIX, CAST_TEXT_EQUAL};

/// Extension trait for appending single-column filter conditions to a [`QueryBuilder`].
///
/// The generic parameter `F` is the caller-defined enum whose variants represent the
/// filterable database columns. Each variant must implement `Into<&'static str>` and `Copy`.
///
/// ## No-Op Conditions
///
/// The clause is **not** appended if any of the following are true:
/// - `filters.filter_by` is `None` (no column selected).
/// - `filters.filter_value` is `None` (no value provided).
/// - The filter value trims to an empty string.
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
///
/// impl From<StatusFilter> for &'static str {
///     fn from(_: StatusFilter) -> Self { "users.active" }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let filters = FilterParams {
///     filter_by: Some(StatusFilter::Active),
///     filter_value: Some("true".into()),
/// };
/// qb.apply_filtering(&filters);
/// // Appended: AND users.active::text = $1
/// ```
pub trait ApplyFiltering<F> {
    /// Appends `AND column::text = $value` when both filter field and value are present.
    ///
    /// Uses a `::text` cast on the column side, making this compatible with UUID, enum,
    /// integer, and boolean columns without requiring explicit type annotation on the client.
    ///
    /// # Arguments
    ///
    /// * `filters` - Reference to incoming [`FilterParams`] containing the optional column
    ///   selector and filter value string.
    fn apply_filtering(&mut self, filters: &FilterParams<F>)
    where
        F: Copy + Into<&'static str>;
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
