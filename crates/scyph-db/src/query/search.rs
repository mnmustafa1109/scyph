//! Full-text search ILIKE query composition.
//!
//! This module provides the [`ApplySearch`] extension trait, which appends a multi-column
//! case-insensitive search clause to a [`QueryBuilder<Postgres>`].
//!
//! ## How It Works
//!
//! When a non-empty search term is present in [`SearchParams`], `apply_search` builds a
//! grouped `AND (col1 ILIKE $n OR col2 ILIKE $n OR ...)` clause where:
//!
//! - The columns are derived by iterating all variants of a user-supplied enum `E` that
//!   implements [`strum::IntoEnumIterator`] and [`Into<&'static str>`].
//! - A single `%term%` wildcard pattern is bound once and reused across all OR branches,
//!   meaning the search term always matches as a substring within any of the listed columns.
//! - Special LIKE metacharacters (`%`, `_`, `\`) in the user-supplied search term are
//!   escaped via [`escape_like_pattern`] before binding, preventing wildcard injection.
//! - If `search.q` is `None`, empty, or only whitespace, no clause is appended and the
//!   query builder is left unmodified.

use scyph_extractors::query::SearchParams;
use sqlx::{Postgres, QueryBuilder};
use strum::IntoEnumIterator;

use super::constants::{
    AND_OPEN_GROUP, CLOSE_GROUP, ILIKE, OR_DELIMITER, WILDCARD, escape_like_pattern,
};

/// Extension trait for appending full-text search ILIKE filters to a [`QueryBuilder`].
///
/// Implemented for `QueryBuilder<Postgres>`. The type parameter `E` is the caller-defined
/// enum whose variants represent the database columns to search. Each variant must be
/// convertible to a `&'static str` column expression (e.g. `"users.name"`, `"posts.title"`).
///
/// ## Column Enum Requirements
///
/// The search column enum `E` must implement:
/// - [`strum::IntoEnumIterator`]: to iterate all variants automatically (derive with `#[derive(EnumIter)]`).
/// - [`Into<&'static str>`]: to map each variant to its SQL column expression.
/// - [`Copy`]: because each variant is consumed during iteration.
///
/// ## Wildcard Behavior
///
/// The search term is wrapped in `%…%` wildcards, so `"ali"` matches `"Alice"`, `"Malia"`,
/// `"Specialist"`, etc. This is a substring match, not a prefix or full-word match.
///
/// ## Empty Enum Guard
///
/// If the enum `E` has no variants (i.e. `E::iter().peek()` is `None`), no clause is
/// appended and the function returns immediately without modifying the query builder.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::ApplySearch;
/// use scyph_extractors::query::SearchParams;
/// use sqlx::{Postgres, QueryBuilder};
/// use strum_macros::EnumIter;
///
/// /// Columns to include in the users search
/// #[derive(Copy, Clone, EnumIter)]
/// enum UserSearch { Name, Email }
///
/// impl From<UserSearch> for &'static str {
///     fn from(s: UserSearch) -> Self {
///         match s {
///             UserSearch::Name  => "users.name",
///             UserSearch::Email => "users.email",
///         }
///     }
/// }
///
/// let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
/// let search = SearchParams { q: Some("alice".into()) };
/// qb.apply_search::<UserSearch>(&search);
/// // Appended: AND (users.name ILIKE $1 OR users.email ILIKE $1)
/// // Bound:    $1 = "%alice%"
/// ```
pub trait ApplySearch {
    /// Appends ILIKE search clauses across all fields defined by enum `E`.
    ///
    /// If `search.q` contains non-whitespace text, appends
    /// `AND (col1 ILIKE $n OR col2 ILIKE $n ...)` where `$n` is the bound `%term%` pattern.
    /// Special LIKE metacharacters in the search term are escaped before binding.
    ///
    /// If `search.q` is `None`, empty, or only whitespace, the query builder is left unchanged.
    ///
    /// # Arguments
    ///
    /// * `search` - Reference to incoming [`SearchParams`] containing the optional search string `q`.
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
