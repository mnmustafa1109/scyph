//! SQL query clause constants and pattern escaping helpers.
//!
//! This module centralizes all raw SQL fragment strings used by the query composition
//! traits in this crate. Using typed constants rather than inline string literals serves
//! two purposes:
//!
//! 1. **Maintainability**: a typo or whitespace error in a SQL fragment is fixed in one place.
//! 2. **Discoverability**: the full vocabulary of SQL constructs used by this crate is visible
//!    at a glance without reading trait implementations.
//!
//! All constants include surrounding spaces where required for valid SQL concatenation.
//! For example, `AND_PREFIX` is `" AND "` (with leading space) so it can be pushed after any
//! previous SQL fragment without a space gap or missing space.
//!
//! ## Injection Safety
//!
//! These constants are pushed via `QueryBuilder::push()` (not `push_bind()`). They are
//! static strings defined at compile time and never derived from user input, so they carry
//! no SQL injection risk. User-supplied values are always handled via `push_bind()` in the
//! trait implementations.

/// SQL constant for AND prefix.
pub const AND_PREFIX: &str = " AND ";

/// SQL constant opening an AND search group.
pub const AND_OPEN_GROUP: &str = " AND (";

/// SQL constant for OR delimiter between search columns.
pub const OR_DELIMITER: &str = " OR ";

/// SQL constant closing a parenthesis group.
pub const CLOSE_GROUP: &str = ")";

/// SQL constant for case-insensitive LIKE comparison.
pub const ILIKE: &str = " ILIKE ";

/// SQL constant for ORDER BY sorting.
pub const ORDER_BY: &str = " ORDER BY ";

/// SQL constant for ascending sort order.
pub const ASCENDING: &str = " ASC";

/// SQL constant for descending sort order.
pub const DESCENDING: &str = " DESC";

/// SQL constant for LIMIT clause.
pub const LIMIT: &str = " LIMIT ";

/// SQL constant for OFFSET clause.
pub const OFFSET: &str = " OFFSET ";

/// SQL constant for text-cast equality comparison (`::text = `).
pub const CAST_TEXT_EQUAL: &str = "::text = ";

/// SQL wildcard symbol for partial pattern matching.
pub const WILDCARD: char = '%';

/// Helper function to escape special SQL LIKE pattern wildcards (`%`, `_`, `\`).
///
/// Ensures user-supplied search strings match literal text rather than behaving as wildcards.
/// Must be called on every user-supplied search string before constructing a `LIKE` or
/// `ILIKE` pattern. The result is then wrapped in `%…%` wildcards by [`ApplySearch`].
///
/// ## Escaped Characters
///
/// | Character | Meaning in LIKE | Escaped as |
/// |-----------|----------------|------------|
/// | `%`       | Matches any sequence of characters | `\%` |
/// | `_`       | Matches any single character | `\_` |
/// | `\`       | Escape character itself | `\\` |
///
/// ## Example
///
/// ```rust
/// use scyph_db::escape_like_pattern;
///
/// assert_eq!(escape_like_pattern("100%"),  "100\\%");
/// assert_eq!(escape_like_pattern("foo_bar"), "foo\\_bar");
/// assert_eq!(escape_like_pattern("C:\\path"), "C:\\\\path");
/// assert_eq!(escape_like_pattern("alice"),   "alice");  // unchanged
/// ```
#[inline]
pub fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
