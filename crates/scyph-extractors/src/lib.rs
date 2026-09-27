#![warn(missing_docs)]
//! # Scyph Extractors
//!
//! `scyph-extractors` provides type-safe, validated request payload extractors and parameter models for Axum web applications.
//!
//! ## Overview
//!
//! This crate aggregates request extraction concerns into modular components:
//! - **[`body`]**: Validated JSON body extractor ([`ValidatedJson`]) combining automatic string sanitization (`sanitizer`) and domain validation (`garde`). Also includes [`SanitizedJson`].
//! - **[`link`]**: Header-aware canonical link generator ([`LinkGenerator`]) and URL builder ([`LinkBuilder`]) supporting reverse proxy headers (`X-Forwarded-Host`, `X-Forwarded-Proto`).
//! - **[`path`]**: Strongly-typed path parameter extractors ([`TypedPath`]) and validated path parameters ([`ValidatedPath`]).
//! - **[`query`]**: Validated query parameter extractor ([`ValidatedQuery`]), request query parameter models ([`RequestParams`], [`PaginationParams`], [`SortParams`], [`FilterParams`], [`SearchParams`]), and string-to-number deserializer ([`deserialize_number_from_string`]).
//! - **[`error`]**: Granular extraction and validation errors ([`ExtractorError`]) converted automatically to RFC 7807 problem details ([`AppError`](scyph_core::AppError)).
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use garde::Validate;
//! use sanitizer::Sanitizer;
//! use scyph_extractors::{body::ValidatedJson, query::ValidatedQuery, query::RequestParams};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Validate, Sanitizer)]
//! pub struct CreateUser {
//!     #[sanitizer(trim, lower_case)]
//!     #[garde(email)]
//!     pub email: String,
//! }
//!
//! async fn create_user_handler(
//!     ValidatedJson(payload): ValidatedJson<CreateUser>,
//! ) -> &'static str {
//!     println!("User email: {}", payload.email);
//!     "Created"
//! }
//! ```

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
