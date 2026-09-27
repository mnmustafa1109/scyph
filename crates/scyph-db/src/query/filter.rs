//! Single-column equality filter composition.

use scyph_extractors::query::FilterParams;
use sqlx::{Postgres, QueryBuilder};

use super::constants::{AND_PREFIX, CAST_TEXT_EQUAL};

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
