//! Core traits and types for authentication, authorization, and permission enforcement.
//!
//! This module defines:
//! - [`Authorizable`]: Marker trait for user role identifiers.
//! - [`Claims`]: Trait for parsing and validating JWT claims payloads.
//! - [`Action`]: Standard enumeration of CRUD and custom actions used in policy evaluation.
//!
//! # Design Philosophy
//!
//! These traits form the type-level contract between the authentication layer (`scyph-auth`)
//! and the authorization layer (`scyph-abac`). By parameterizing over `Claims` everywhere,
//! the framework remains fully generic — applications define their own role enums and claims
//! structs without any hard-coded types in library code.
//!
//! The typical dependency graph looks like:
//!
//! ```text
//! Your App
//!   ├── defines MyClaims (implements Claims)
//!   ├── defines MyRole   (implements Authorizable automatically)
//!   ├── scyph-auth  uses Claims + Authorizable for JWT extraction / RBAC
//!   └── scyph-abac  uses Claims + Action for policy evaluation / SQL filtering
//! ```
//!
//! # Quick Start
//!
//! Define your role enum and claims struct, implement [`Claims`], and plug them into
//! the Scyph middleware:
//!
//! ```rust
//! use scyph_core::{Claims, Action};
//! use uuid::Uuid;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
//! enum UserRole { Guest, User, Moderator, Admin }
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! struct AppClaims {
//!     sub: Uuid,
//!     exp: i64,
//!     jti: String,
//!     role: UserRole,
//!     // application-specific fields:
//!     email: String,
//!     tenant_id: Uuid,
//! }
//!
//! impl Claims for AppClaims {
//!     type Role = UserRole;
//!     fn subject(&self) -> Uuid     { self.sub }
//!     fn expiry(&self) -> i64       { self.exp }
//!     fn jti(&self) -> &str         { &self.jti }
//!     fn role(&self) -> &UserRole   { &self.role }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

/// Marker trait representing a role or permission identifier usable for authorization.
///
/// Automatically implemented for any type that satisfies `Copy + Eq + Send + Sync + 'static`.
pub trait Authorizable: Copy + Eq + Send + Sync + 'static {}
impl<T> Authorizable for T where T: Copy + Eq + Send + Sync + 'static {}

/// Core trait to implement on your application's custom JWT claims struct.
///
/// Implementing `Claims` allows Scyph's authentication middleware and extractors
/// to read the user subject ID (`sub`), expiration timestamp (`exp`), unique JWT ID (`jti`),
/// and role type (`Role`).
///
/// # Examples
///
/// A minimal realistic implementation with application-specific fields:
///
/// ```rust
/// use scyph_core::Claims;
/// use uuid::Uuid;
/// use serde::{Serialize, Deserialize};
///
/// /// Application role enum. Implements [`scyph_core::Authorizable`] automatically
/// /// because it is `Copy + Eq + Send + Sync + 'static`.
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// enum UserRole { Guest, User, Moderator, Admin }
///
/// /// Full JWT claims payload embedded in every access token.
/// ///
/// /// Fields `sub`, `exp`, and `jti` are standard JWT registered claims.
/// /// `role`, `email`, and `tenant_id` are application-specific private claims.
/// #[derive(Clone, Serialize, Deserialize)]
/// struct AppClaims {
///     /// Subject — the authenticated user's UUID.
///     sub: Uuid,
///     /// Expiration — Unix timestamp (seconds since epoch).
///     exp: i64,
///     /// JWT ID — unique per-token identifier for revocation support.
///     jti: String,
///     /// Role — controls RBAC and ABAC policy decisions.
///     role: UserRole,
///     /// Application-specific: user's email address.
///     email: String,
///     /// Application-specific: tenant scope for multi-tenant APIs.
///     tenant_id: Uuid,
/// }
///
/// impl Claims for AppClaims {
///     type Role = UserRole;
///
///     fn subject(&self) -> Uuid       { self.sub }
///     fn expiry(&self) -> i64         { self.exp }
///     fn jti(&self) -> &str           { &self.jti }
///     fn role(&self) -> &UserRole     { &self.role }
///
///     // Optional: override is_expired if you use millisecond timestamps
///     // fn is_expired(&self) -> bool { self.exp < chrono::Utc::now().timestamp_millis() }
/// }
///
/// // Construct claims for testing
/// let claims = AppClaims {
///     sub: Uuid::now_v7(),
///     exp: chrono::Utc::now().timestamp() + 3600,
///     jti: "unique-token-id-001".into(),
///     role: UserRole::Admin,
///     email: "alice@example.com".into(),
///     tenant_id: Uuid::now_v7(),
/// };
///
/// assert_eq!(*claims.role(), UserRole::Admin);
/// assert!(!claims.is_expired());
/// ```
pub trait Claims: serde::de::DeserializeOwned + Serialize + Send + Sync + Clone + 'static {
    /// The associated role type used by your application for access control.
    type Role: Authorizable;

    /// Returns the unique subject identifier (e.g. User UUID) for the token holder.
    fn subject(&self) -> Uuid;

    /// Returns the token expiration time as a Unix timestamp (in seconds).
    fn expiry(&self) -> i64;

    /// Returns the unique JWT identifier string (`jti`) used for revocation checking.
    fn jti(&self) -> &str;

    /// Returns a reference to the user's role.
    fn role(&self) -> &Self::Role;

    /// Returns `true` if the token's expiration timestamp is in the past relative to UTC now.
    ///
    /// Defaults to checking if `chrono::Utc::now().timestamp() > self.expiry()`.
    /// Override this method if your claims use non-standard epoch representations.
    ///
    /// # Notes
    ///
    /// - The default implementation uses **second-precision** Unix timestamps. If your JWT
    ///   library stores `exp` in milliseconds, override this method accordingly.
    /// - This method is intentionally a fallback. Scyph's JWT verification layer
    ///   ([`scyph_auth::verify_token`]) performs expiry validation during decoding via the
    ///   `jsonwebtoken` crate. `is_expired` is provided for secondary in-process checks
    ///   (e.g. guarding cached claims that may have been stored before the token expired).
    /// - Clock skew between services is not compensated. Add a small leeway at the token
    ///   issuance layer if needed (e.g. set `exp = now + ttl + 30` seconds).
    fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expiry()
    }
}

/// Authorization actions used in Attribute-Based Access Control (ABAC) or Policy evaluation.
///
/// `Action` represents the **operation** a subject is attempting on a resource. It is the
/// central axis of ABAC policy decisions, alongside the subject (who) and resource (what).
///
/// Marked as `#[non_exhaustive]` to allow future action extensions via [`Action::Custom`]
/// without breaking existing match patterns in downstream crates.
///
/// # ABAC Usage Context
///
/// In `scyph-abac`, actions are passed to [`AbacPolicy::check`] to determine whether a
/// subject may perform a given operation:
///
/// ```rust,ignore
/// use scyph_core::Action;
/// use scyph_abac::AbacPolicy;
///
/// impl AbacPolicy for DocumentPolicy {
///     type Resource = Document;
///     type Claims = AppClaims;
///
///     fn check(subject: &AuthUser<AppClaims>, doc: &Document, action: Action)
///         -> Result<(), AbacError>
///     {
///         match action {
///             Action::Read   => check_read_permission(subject, doc),
///             Action::Update => check_ownership(subject, doc),
///             Action::Delete => check_admin_or_owner(subject, doc),
///             Action::Custom(ref name) if name == "publish" => check_publish(subject, doc),
///             _ => Ok(()), // allow by default for unhandled actions
///         }
///     }
/// }
/// ```
///
/// # Custom Actions
///
/// Use [`Action::Custom`] for domain-specific operations that don't map to standard CRUD:
///
/// ```rust
/// use scyph_core::Action;
/// use std::borrow::Cow;
///
/// let publish = Action::Custom(Cow::Borrowed("publish"));
/// let export  = Action::Custom(Cow::Borrowed("export"));
/// let approve = Action::Custom(Cow::Borrowed("approve"));
///
/// assert_ne!(publish, Action::Create);
/// assert_eq!(publish, Action::Custom(Cow::Borrowed("publish")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Action {
    /// Create a new entity or resource.
    Create,
    /// Read or view an existing entity or resource.
    Read,
    /// Update or modify an existing entity or resource.
    Update,
    /// Delete or remove an existing entity or resource.
    Delete,
    /// List or search collections of resources.
    List,
    /// Custom domain-specific action identified by a static string slice.
    Custom(Cow<'static, str>),
}
