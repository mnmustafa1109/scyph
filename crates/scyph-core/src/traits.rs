//! Core traits and types for authentication, authorization, and permission enforcement.
//!
//! This module defines:
//! - [`Authorizable`]: Marker trait for user role identifiers.
//! - [`Claims`]: Trait for parsing and validating JWT claims payloads.
//! - [`Action`]: Standard enumeration of CRUD and custom actions used in policy evaluation.

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
/// ```rust
/// use scyph_core::Claims;
/// use uuid::Uuid;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// enum UserRole { User, Admin }
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct MyClaims {
///     sub: Uuid,
///     exp: i64,
///     jti: String,
///     role: UserRole,
/// }
///
/// impl Claims for MyClaims {
///     type Role = UserRole;
///
///     fn subject(&self) -> Uuid { self.sub }
///     fn expiry(&self) -> i64  { self.exp }
///     fn jti(&self) -> &str    { &self.jti }
///     fn role(&self) -> &Self::Role { &self.role }
/// }
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
    fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expiry()
    }
}

/// Authorization actions used in Attribute-Based Access Control (ABAC) or Policy evaluation.
///
/// Marked as `#[non_exhaustive]` to allow future action extensions via [`Action::Custom`]
/// without breaking existing match patterns in downstream crates.
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
