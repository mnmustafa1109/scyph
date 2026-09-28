//! High-performance Base64 URL-safe UUID cursor pagination helpers.
//!
//! This module provides two approaches to pagination for Axum APIs:
//!
//! ## Cursor-Based Pagination ([`Cursor`] + [`PageParams`])
//!
//! Cursor pagination is the recommended approach for large, frequently updated datasets.
//! Rather than `OFFSET N`, the database query uses `WHERE id > $cursor_uuid ORDER BY id`,
//! which is index-efficient and produces stable results even as rows are inserted or deleted.
//!
//! The opaque cursor is a Base64 URL-safe encoded [`uuid::Uuid`], making it safe to embed in
//! URLs without percent-encoding.
//!
//! ```rust
//! use scyph_utils::pagination::{Cursor, PageParams};
//! use uuid::Uuid;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Product { id: Uuid, name: String }
//!
//! // Encode the last-item UUID into an opaque cursor token
//! let last_id = Uuid::nil();
//! let cursor_token = Cursor::encode(last_id);
//!
//! // Decode back to UUID for use as SQL parameter
//! let decoded_id = Cursor::decode(&cursor_token).unwrap();
//! assert_eq!(decoded_id, last_id);
//! ```
//!
//! ## Building Paginated Responses
//!
//! Use [`Cursor::build_page`] to automate the "fetch N+1 items, pop the extra" pattern:
//!
//! ```rust
//! use scyph_utils::pagination::Cursor;
//! use serde::Serialize;
//! use uuid::Uuid;
//!
//! #[derive(Serialize)]
//! struct Item { id: Uuid, name: String }
//!
//! let mut items = vec![
//!     Item { id: Uuid::new_v4(), name: "Alpha".into() },
//!     Item { id: Uuid::new_v4(), name: "Beta".into() },
//! ];
//!
//! // Limit=20, fetched 2 items → no next page
//! let page = Cursor::build_page(&mut items, 20, |item| item.id);
//! assert!(!page.has_next);
//! ```
//!
//! ## [`PaginationError`]
//!
//! Returned when a client provides a cursor string that cannot be decoded as a valid
//! Base64 UUID. Converts automatically to HTTP 400 Bad Request via `From<PaginationError> for AppError`.

pub mod cursor;
pub mod error;
pub mod params;

pub use cursor::Cursor;
pub use error::PaginationError;
pub use params::PageParams;
