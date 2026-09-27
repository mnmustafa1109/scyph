//! SQL query clause constants and pattern escaping helpers.

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
#[inline]
pub fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
