//! Full-text search ILIKE query composition.

use scyph_extractors::query::SearchParams;
use sqlx::{Postgres, QueryBuilder};
use strum::IntoEnumIterator;

use super::constants::{
    AND_OPEN_GROUP, CLOSE_GROUP, ILIKE, OR_DELIMITER, WILDCARD, escape_like_pattern,
};

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
