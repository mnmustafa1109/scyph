//! # Scyph Auth
//!
//! `scyph-auth` provides authentication and authorization utilities for Axum applications.
//!
//! ## Key Modules
//! - **[`cache`]**: High-performance in-memory caching for user profiles and JWT token revocation (blacklisting).
//! - **[`extractor`]**: Axum request extractor (`AuthUser`) for extracting authenticated users from request headers.
//! - **[`jwt`]**: Signing and verifying JSON Web Tokens (JWT).
//! - **[`password`]**: Secure password hashing and verification using Argon2id.
//! - **[`middleware`]**: Axum middleware for enforcing Role-Based Access Control (RBAC).
//! - **[`rbac`]**: Predicate-based function helper for checking user roles in handlers.
//! - **[`router_ext`]**: Extension trait ([`RoleRouterExt`]) to protect router routes easily.

#![warn(missing_docs)]

/// In-memory profile caching and token revocation management using Moka.
pub mod cache;

/// Axum `FromRequestParts` extractors for authenticated user state.
pub mod extractor;

/// JSON Web Token (JWT) creation, signature verification, and decoding.
pub mod jwt;

/// Role-based access control middleware for Axum routes.
pub mod middleware;

/// Argon2id password hashing and constant-time signature verification.
pub mod password;

/// Helper functions for checking user authorization roles.
pub mod rbac;

/// Fluent extension traits for Axum [`Router`](axum::Router).
pub mod router_ext;

#[doc(inline)]
pub use cache::AuthCacheService;

#[doc(inline)]
pub use extractor::{AuthExtractorState, AuthUser};

#[doc(inline)]
pub use jwt::{JwtError, create_token, verify_token};

#[doc(inline)]
pub use middleware::{AllowedRoles, require_roles_layer};

#[doc(inline)]
pub use password::{PasswordError, hash_password, verify_password};

#[doc(inline)]
pub use rbac::require_role;

#[doc(inline)]
pub use router_ext::RoleRouterExt;
