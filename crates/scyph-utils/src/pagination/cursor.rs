//! Base64 URL-safe UUID cursor encoding and decoding.

use crate::pagination::error::PaginationError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
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

    /// Automatically builds a [`PagedResponse`](scyph_core::PagedResponse) envelope from a fetched item vector.
    ///
    /// Inspects the item list length against `limit`:
    /// - If `items.len() > limit`, pops the extra element (fetched to detect `has_next`),
    /// - Encodes the next cursor using `get_id(&last_item)`,
    /// - Returns a clean [`PagedResponse`](scyph_core::PagedResponse).
    ///
    /// # Arguments
    ///
    /// * `items` - Mutable vector of fetched items (typically queried with `LIMIT limit + 1`).
    /// * `limit` - Target page size limit.
    /// * `get_id` - Closure returning the [`Uuid`] cursor identifier from an item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_utils::pagination::Cursor;
    /// use serde::Serialize;
    /// use uuid::Uuid;
    ///
    /// #[derive(Serialize)]
    /// struct User { id: Uuid, name: String }
    ///
    /// let mut users = vec![
    ///     User { id: Uuid::nil(), name: "Alice".into() },
    /// ];
    ///
    /// let paged = Cursor::build_page(&mut users, 20, |u| u.id);
    /// assert_eq!(paged.data.len(), 1);
    /// ```
    pub fn build_page<T, F>(
        items: &mut Vec<T>,
        limit: i64,
        get_id: F,
    ) -> scyph_core::PagedResponse<T>
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
        let mut resp = scyph_core::PagedResponse::new(std::mem::take(items), count, 1, limit);
        resp.has_next = has_next;
        if let Some(nc) = next_cursor {
            resp = resp.with_next_cursor(nc);
        }
        resp
    }
}
