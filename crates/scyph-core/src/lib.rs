//! # Scyph Core
//!
//! `scyph-core` provides the foundational building blocks for web applications built
//! with the Scyph framework.
//!
//! ## Overview
//!
//! This crate includes:
//! - **[`error`]**: Standardized RFC 7807 Problem Details HTTP error handling via [`AppError`].
//! - **[`response`]**: Standardized API response envelopes ([`ApiResponse`] and [`PagedResponse`]).
//! - **[`traits`]**: Traits for user identity, JWT claims parsing ([`Claims`]), and authorization actions ([`Action`]).
//!
//! ## Quick Example
//!
//! ```rust
//! use scyph_core::{ApiResponse, AppError};
//! use axum::http::StatusCode;
//!
//! async fn example_handler() -> Result<ApiResponse<&'static str>, AppError> {
//!     Ok(ApiResponse::ok("Hello, Scyph!"))
//! }
//! ```

#![warn(missing_docs)]

/// Error handling definitions and HTTP response mapping.
pub mod error;

/// Standardized JSON response envelopes and pagination models.
pub mod response;

/// Traits and enums for authentication, authorization, and claims handling.
pub mod traits;

#[doc(inline)]
pub use error::{AppError, ErrorDetails};

#[doc(inline)]
pub use response::{ApiResponse, PagedResponse, ResponseMeta, Result};

#[doc(inline)]
pub use traits::{Action, Authorizable, Claims};
