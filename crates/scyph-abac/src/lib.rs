#![warn(missing_docs)]
//! # Scyph ABAC
//!
//! `scyph-abac` provides Attribute-Based Access Control (ABAC) policies, SQL query filter
//! injections for SQLx, and optional integration with the AWS Cedar Policy Engine.
//!
//! ## Overview
//!
//! - **[`policy`]**: Defines the core [`AbacPolicy`] trait for domain-level authorization and SQL filter generation.
//! - **[`filter`]**: Provides [`FilterBuilder`], a SQLx wrapper for building parameterized SQL `WHERE` clauses dynamically.
//! - **[`ext`]**: Extension trait [`AuthUserEnforceExt`] for fluent `.enforce(...)` checks on `AuthUser`.
//! - **[`error`]**: Defines [`AbacError`], the error type for ABAC policy enforcement and Cedar engine evaluation.
//! - **[`cedar`]**: (Optional feature `cedar`) AWS Cedar Policy Engine authorization with [`CedarAuthorizer`] and [`IntoCedarEntity`].
//!
//! ## ABAC vs RBAC
//!
//! Role-Based Access Control (RBAC, via `scyph-auth`) checks whether a user has a certain role.
//! Attribute-Based Access Control (ABAC, this crate) checks whether a user may perform a specific
//! **action** on a specific **resource instance** based on attributes of both the user and the resource.
//!
//! Example: RBAC can say "only Admins can access `/documents`". ABAC can say "a User can `Read`
//! a Document if they own it OR if the document is public, but can only `Delete` it if they own it".
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AuthUserEnforceExt};
//! use scyph_core::Action;
//!
//! // 1. Define a policy for your resource
//! struct DocumentPolicy;
//! impl AbacPolicy for DocumentPolicy {
//!     type Resource = Document;
//!     type Claims = AppClaims;
//!
//!     fn check(user: &AuthUser<AppClaims>, doc: &Document, action: Action)
//!         -> Result<(), AbacError>
//!     {
//!         if doc.owner_id == user.id { return Ok(()); }
//!         Err(AbacError::Forbidden("Access denied".into()))
//!     }
//! }
//!
//! // 2. Enforce in your handler
//! async fn get_doc(user: AuthUser<AppClaims>, doc: Document) -> Result<ApiResponse<Document>, AppError> {
//!     user.enforce::<DocumentPolicy>(&doc, Action::Read)?;
//!     Ok(ApiResponse::ok(doc))
//! }
//! ```

/// ABAC error type definitions.
pub mod error;

/// Extension traits adding `.enforce(...)` directly to [`scyph_auth::AuthUser`].
///
/// Use this module for inline, fluent policy enforcement inside handler functions.
pub mod ext;

/// SQLx [`sqlx::QueryBuilder`] wrapper for safe, parameterized ABAC SQL query filtering.
///
/// Use this module when building dynamic `WHERE` clauses based on user attributes.
pub mod filter;

/// Core [`AbacPolicy`] trait definition for domain-level authorization rules.
///
/// Implement this trait on a policy struct for each protected resource type in your domain.
pub mod policy;

#[cfg(feature = "cedar")]
pub mod cedar;

pub use error::AbacError;
pub use ext::AuthUserEnforceExt;
pub use filter::FilterBuilder;
pub use policy::AbacPolicy;

#[cfg(feature = "cedar")]
pub use cedar::{
    CedarAuthorizer, CedarEntityBuilder, CedarError, CedarSchemaBuilder, CedarType, IntoCedarEntity,
};
