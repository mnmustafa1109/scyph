#![warn(missing_docs)]
//! # Scyph Extractors
//!
//! `scyph-extractors` provides type-safe, validated request payload extractors for Axum backends.
//!
//! ## Modules
//!
//! - **[`body`]**: Validated and sanitized JSON body extractors ([`ValidatedJson`], [`SanitizedJson`]).
//! - **[`path`]**: Strongly-typed and validated URL path parameter extractors ([`TypedPath`], [`ValidatedPath`]).
//! - **[`query`]**: Validated URL query string parameter extractors ([`ValidatedQuery`]).
//! - **[`error`]**: Granular extractor errors mapped to RFC 7807 problem details ([`ExtractorError`]).

/// Request body extractors with input sanitization and `garde` validation.
pub mod body;

/// Granular request extraction and validation error types.
pub mod error;

/// URL path parameter extractors.
pub mod path;

/// Query string parameter extractors with `garde` validation.
pub mod query;

#[doc(inline)]
pub use body::{SanitizedJson, ValidatedJson};

#[doc(inline)]
pub use error::ExtractorError;

#[doc(inline)]
pub use path::{TypedPath, ValidatedPath};

#[doc(inline)]
pub use query::ValidatedQuery;
