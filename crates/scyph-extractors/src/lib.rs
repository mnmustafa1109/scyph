#![warn(missing_docs)]
//! # Scyph Extractors
//!
//! `scyph-extractors` provides type-safe, validated request payload extractors and parameter models for Axum backends.
//!
//! ## Modules
//!
//! - **[`body`]**: Validated and sanitized JSON body extractors ([`ValidatedJson`], [`SanitizedJson`]).
//! - **[`link`]**: Header-aware canonical URL and link generator ([`LinkGenerator`], [`LinkBuilder`]).
//! - **[`path`]**: Strongly-typed and validated URL path parameter extractors ([`TypedPath`], [`ValidatedPath`]).
//! - **[`query`]**: Validated URL query string parameter extractors ([`ValidatedQuery`]), request parameter models ([`RequestParams`], [`PaginationParams`]), and number deserializers ([`deserialize_number_from_string`]).
//! - **[`error`]**: Granular extractor errors mapped to RFC 7807 problem details ([`ExtractorError`]).

/// Request body extractors with input sanitization and `garde` validation.
pub mod body;

/// Granular request extraction and validation error types.
pub mod error;

/// Header-aware canonical link and URL builder.
pub mod link;

/// URL path parameter extractors.
pub mod path;

/// Query string parameter extractors with `garde` validation and query models.
pub mod query;

#[doc(inline)]
pub use body::{SanitizedJson, ValidatedJson};

#[doc(inline)]
pub use error::ExtractorError;

#[doc(inline)]
pub use link::{LinkBuilder, LinkGenerator};

#[doc(inline)]
pub use path::{TypedPath, ValidatedPath};

#[doc(inline)]
pub use query::{
    FilterParams, PaginationParams, RequestParams, SearchParams, SortOrder, SortParams,
    ValidatedQuery, deserialize_number_from_string,
};
