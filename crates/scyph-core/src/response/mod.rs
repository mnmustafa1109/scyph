//! Standardized JSON response envelopes and pagination models.
//!
//! This module provides consistent API response structures across Scyph applications:
//! - [`ApiResponse<T>`]: General envelope for single objects, lists, or custom messages.
//! - [`PagedResponse<T>`]: Pagination envelope containing item lists and pagination metadata.
//! - [`ResponseMeta`]: Standardized metadata block containing latency, trace ID, timestamp, and version.
//!
//! # Examples
//!
//! ```rust
//! use scyph_core::{ApiResponse, ResponseMeta};
//!
//! let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 5, "0.1.0");
//! let response = ApiResponse::ok("Success data").with_meta(meta);
//! assert!(response.success);
//! assert_eq!(response.message, "OK");
//! assert!(response.meta.is_some());
//! ```

pub mod api;
pub mod meta;
pub mod paged;

use crate::AppError;

/// Standardized [`Result`](std::result::Result) type alias defaulting the error type to [`AppError`].
pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[doc(inline)]
pub use api::ApiResponse;

#[doc(inline)]
pub use meta::ResponseMeta;

#[doc(inline)]
pub use paged::PagedResponse;
