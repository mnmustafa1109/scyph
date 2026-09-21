//! # Scyph
//!
//! `scyph` is a modular framework for building production-grade web services with Axum.
//!
//! ## Feature Flags
//!
//! - `auth`: Enables authentication (JWT, Argon2id, Moka caching, RBAC).
//! - `abac`: Enables Attribute-Based Access Control (SQL FilterBuilder, Cedar engine).
//! - `db`: Enables PostgreSQL pool management, transaction helpers, and migrations.

#![warn(missing_docs)]

/// Core foundational building blocks (RFC 7807 AppError, ApiResponse, Claims).
pub use scyph_core as core;

#[cfg(feature = "auth")]
/// Authentication and role-based access control.
pub use scyph_auth as auth;

#[cfg(feature = "abac")]
/// Attribute-based access control policies and Cedar policy engine integration.
pub use scyph_abac as abac;

#[cfg(feature = "db")]
/// Database connection pooling and transaction lifecycle helpers.
pub use scyph_db as db;

// ── Convenient Top-Level Re-exports ──────────────────────────────────────────
pub use scyph_core::{Action, ApiResponse, AppError, Authorizable, Claims, PagedResponse, Result};

#[cfg(feature = "auth")]
pub use scyph_auth::AuthUser;

#[cfg(feature = "abac")]
pub use scyph_abac::{AbacPolicy, FilterBuilder};
