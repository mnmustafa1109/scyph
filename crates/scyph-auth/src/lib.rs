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
//! - **[`error`]**: Authentication and authorization error types ([`AuthError`]).
//!
//! ## Authentication Flow
//!
//! The typical end-to-end authentication flow in a Scyph application:
//!
//! ```text
//! Client
//!   │  POST /auth/login  { email, password }
//!   ▼
//! Login Handler
//!   ├── verify_password_async(&input_pw, &stored_hash)
//!   ├── create_token(&claims, &jwt_secret)
//!   └── returns JWT string to client
//!
//! Client
//!   │  GET /protected  Authorization: Bearer <token>
//!   ▼
//! require_roles_layer middleware
//!   ├── AuthUser::from_request_parts (parses + verifies JWT)
//!   ├── AuthCacheService::is_token_revoked (checks blacklist)
//!   ├── checks role in AllowedRoles
//!   └── calls next handler with AuthUser in extensions
//!
//! Protected Handler
//!   ├── AuthUser<MyClaims> injected as parameter
//!   └── optionally: user.enforce::<MyPolicy>(&resource, Action::Read)
//! ```
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use scyph_auth::{
//!     AuthCacheService, AuthExtractorState, AuthUser,
//!     create_token, hash_password_async, verify_password_async,
//!     RoleRouterExt,
//! };
//! use secrecy::SecretString;
//! use std::sync::Arc;
//!
//! // 1. Build your app state implementing AuthExtractorState
//! #[derive(Clone)]
//! struct AppState {
//!     jwt_secret: SecretString,
//!     cache: Arc<AuthCacheService<UserProfile>>,
//! }
//!
//! // 2. Hash passwords at registration
//! let hash = hash_password_async(&SecretString::from("hunter2")).await?;
//!
//! // 3. Issue tokens at login
//! let token = create_token(&claims, &state.jwt_secret)?;
//!
//! // 4. Protect routes with roles
//! let admin_routes = Router::new()
//!     .route("/admin", get(admin_handler))
//!     .require_roles::<AppClaims>(state.clone(), &[UserRole::Admin]);
//! ```

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

/// Authentication and authorization error definitions.
pub mod error;

#[doc(inline)]
pub use cache::AuthCacheService;

#[doc(inline)]
pub use error::AuthError;

#[doc(inline)]
pub use extractor::{AuthExtractorState, AuthUser, OptionalAuthUser};

#[doc(inline)]
pub use jwt::{JwtError, create_token, verify_token};

#[doc(inline)]
pub use middleware::{AllowedRoles, require_roles_layer};

#[doc(inline)]
pub use password::{
    PasswordError, PasswordService, hash_password, hash_password_async, verify_password,
    verify_password_async,
};

#[doc(inline)]
pub use rbac::require_role;

#[doc(inline)]
pub use router_ext::RoleRouterExt;
