//! Validated query string extractors and reusable parameter models for Axum handlers.
//!
//! ## Components
//!
//! ### [`ValidatedQuery<T>`](extractor::ValidatedQuery)
//!
//! Axum extractor that parses query parameters into `T` (via `serde`) and applies `garde`
//! validation rules. Returns HTTP 422 with per-field error details on validation failure.
//!
//! ```rust,ignore
//! use garde::Validate;
//! use scyph_extractors::query::ValidatedQuery;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Validate)]
//! struct SearchFilter {
//!     #[garde(length(min = 1, max = 200))]
//!     pub q: String,
//! }
//!
//! async fn search(ValidatedQuery(filter): ValidatedQuery<SearchFilter>) {
//!     println!("Searching: {}", filter.q);
//! }
//! ```
//!
//! ### Standard Parameter Models
//!
//! Pre-built composable structs for common pagination, sorting, filtering, and search patterns:
//!
//! | Type | Fields | Use With |
//! |------|--------|----------|
//! | [`PaginationParams`](models::PaginationParams) | `page`, `limit` | Offset pagination |
//! | [`SortParams<S>`](models::SortParams) | `sort_by`, `sort_order` | Column sorting |
//! | [`FilterParams<F>`](models::FilterParams) | `filter_by`, `filter_value` | Column filtering |
//! | [`SearchParams`](models::SearchParams) | `q` | Full-text ILIKE search |
//! | [`RequestParams<S,F>`](models::RequestParams) | All of the above (flattened) | Combined handler queries |
//!
//! ### [`RequestParams<S, F>`](models::RequestParams) — Composite Model
//!
//! ```rust,ignore
//! use scyph_extractors::query::{RequestParams, ValidatedQuery};
//!
//! async fn list_users(
//!     ValidatedQuery(params): ValidatedQuery<RequestParams<UserSort, UserFilter>>,
//! ) {
//!     let page = params.pagination.page;    // u32
//!     let limit = params.pagination.limit;  // u32
//!     let query = params.search.q;          // Option<String>
//!     let sort = params.sort.sort_by;       // Option<UserSort>
//!     let filter = params.filter.filter_by; // Option<UserFilter>
//! }
//! ```
//!
//! Pairs directly with `scyph-db`'s `ApplyRequestParams::apply_request_params` for
//! zero-boilerplate database query composition.

pub mod extractor;
pub mod models;

pub use extractor::ValidatedQuery;
pub use models::{
    FilterParams, PaginationParams, RequestParams, SearchParams, SortOrder, SortParams,
    deserialize_number_from_string,
};
