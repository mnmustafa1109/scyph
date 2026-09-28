//! # Scyph Core
//!
//! `scyph-core` provides the foundational building blocks for web applications built
//! with the Scyph framework. It is deliberately minimal — containing only types,
//! traits, and response envelopes — with no web-framework dependencies beyond Axum.
//!
//! ## Overview
//!
//! This crate includes:
//! - **[`error`]**: Standardized RFC 7807 Problem Details HTTP error handling via [`AppError`].
//! - **[`response`]**: Standardized API response envelopes ([`ApiResponse`] and [`PagedResponse`]).
//! - **[`traits`]**: Traits for user identity, JWT claims parsing ([`Claims`]), and authorization actions ([`Action`]).
//!
//! ## Architecture
//!
//! `scyph-core` sits at the bottom of the Scyph dependency graph. All other crates depend on
//! it, but it depends on nothing Scyph-specific:
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │               Your Application              │
//! └──────────┬─────────────┬────────────────────┘
//!            │             │
//!     ┌──────▼──────┐  ┌───▼──────┐
//!     │ scyph-auth  │  │scyph-abac│
//!     └──────┬──────┘  └───┬──────┘
//!            └──────┬──────┘
//!             ┌─────▼─────┐
//!             │scyph-core │  ← you are here
//!             └───────────┘
//! ```
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
//!
//! ## Error Handling Pattern
//!
//! Define handlers that return `Result<ApiResponse<T>, AppError>`. Axum will automatically
//! call `IntoResponse` on the error path, producing an RFC 7807 JSON error body:
//!
//! ```rust
//! use scyph_core::{ApiResponse, AppError};
//!
//! async fn get_user(id: u64) -> Result<ApiResponse<String>, AppError> {
//!     if id == 0 {
//!         return Err(AppError::NotFound("User not found".into()));
//!     }
//!     Ok(ApiResponse::ok_msg("Alice".into(), "User fetched successfully"))
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
