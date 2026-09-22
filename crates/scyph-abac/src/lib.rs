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
//! - **[`cedar`]**: (Optional feature `cedar`) AWS Cedar Policy Engine authorization with [`CedarAuthorizer`] and [`IntoCedarEntity`].

pub mod error;
pub mod ext;
pub mod filter;
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
