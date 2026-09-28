//! Base64 URL-safe UUID cursor encoding and decoding.

use crate::pagination::error::PaginationError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use scyph_core::PagedResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque cursor wrapper around a 128-bit [`Uuid`] primary key or timestamp identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cursor {
    /// Target UUID identifier.
    pub id: Uuid,
}

impl Cursor {
    /// Encodes a [`Uuid`] into an opaque, URL-safe Base64 cursor string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_utils::pagination::Cursor;
    /// use uuid::Uuid;
    ///
    /// let id = Uuid::nil();
    /// let encoded = Cursor::encode(id);
    /// assert_eq!(Cursor::decode(&encoded).unwrap(), id);
    /// ```
    pub fn encode(id: Uuid) -> String {
        URL_SAFE_NO_PAD.encode(id.as_bytes())
    }

    /// Decodes an opaque URL-safe Base64 cursor string back into a [`Uuid`].
    ///
    /// # Errors
    ///
    /// Returns [`PaginationError::InvalidCursor`] if Base64 decoding fails or byte array is invalid.
    pub fn decode(s: &str) -> Result<Uuid, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|e| PaginationError::InvalidCursor(e.to_string()))?;

        Uuid::from_slice(&bytes).map_err(|e| PaginationError::InvalidCursor(e.to_string()))
    }

    /// Automatically builds a [`PagedResponse`] envelope from a fetched item vector.
    ///
    /// Inspects the item list length against `limit`:
    /// - If `items.len() > limit`, pops the extra element (fetched to detect `has_next`),
    /// - Encodes the next cursor using `get_id(&last_item)`,
    /// - Returns a clean [`PagedResponse`].
    ///
    /// ## "Limit + 1" Pattern
    ///
    /// The standard cursor pagination pattern is to query `LIMIT limit + 1` rows from the database.
    /// If more than `limit` rows are returned, there is a next page. `build_page` implements this
    /// pattern automatically: pass it `limit` (the user-requested page size) and a vector fetched
    /// with `LIMIT limit + 1`. It truncates to `limit`, sets `has_next`, and encodes the cursor
    /// from the **last item in the page** (not the extra row).
    ///
    /// ## Database Query Pattern
    ///
    /// ```sql
    /// -- Fetch one extra row to determine if a next page exists
    /// SELECT id, name, created_at FROM users
    /// WHERE id > $1           -- cursor_id (None = first page)
    /// ORDER BY id ASC
    /// LIMIT $2 + 1            -- limit + 1
    /// ```
    ///
    /// # Arguments
    ///
    /// * `items` - Mutable vector of fetched items (typically queried with `LIMIT limit + 1`).
    /// * `limit` - Target page size limit (automatically clamped to `[1, 100]`).
    /// * `get_id` - Closure returning the [`Uuid`] cursor identifier from an item reference.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_utils::pagination::{Cursor, PageParams};
    /// use serde::Serialize;
    /// use uuid::Uuid;
    ///
    /// #[derive(Serialize)]
    /// struct User { id: Uuid, name: String }
    ///
    /// // Simulating: DB returned 21 rows for a page size of 20 (limit + 1 pattern)
    /// let mut users: Vec<User> = (0..21u8)
    ///     .map(|i| User { id: Uuid::nil(), name: format!("User {i}") })
    ///     .collect();
    ///
    /// let page = Cursor::build_page(&mut users, 20, |u| u.id);
    ///
    /// // Page contains exactly 20 items
    /// assert_eq!(page.data.len(), 20);
    /// // has_next is true because 21 > 20
    /// assert!(page.has_next);
    /// // next_cursor is set to the encoded UUID of the 20th item
    /// assert!(page.next_cursor.is_some());
    ///
    /// // First page (only 5 rows returned — no next page)
    /// let mut small: Vec<User> = (0..5u8)
    ///     .map(|i| User { id: Uuid::nil(), name: format!("User {i}") })
    ///     .collect();
    /// let last_page = Cursor::build_page(&mut small, 20, |u| u.id);
    /// assert!(!last_page.has_next);
    /// assert!(last_page.next_cursor.is_none());
    /// ```
    pub fn build_page<T, F>(items: &mut Vec<T>, limit: i64, get_id: F) -> PagedResponse<T>
    where
        T: Serialize,
        F: Fn(&T) -> Uuid,
    {
        let max_limit = limit.clamp(1, 100) as usize;
        let has_next = items.len() > max_limit;

        if has_next {
            items.truncate(max_limit);
        }

        let next_cursor = if has_next {
            items.last().map(|item| Self::encode(get_id(item)))
        } else {
            None
        };

        let count = items.len() as i64;
        let mut resp = PagedResponse::new(std::mem::take(items), count, 1, limit);
        resp.has_next = has_next;
        if let Some(nc) = next_cursor {
            resp = resp.with_next_cursor(nc);
        }
        resp
    }
}
