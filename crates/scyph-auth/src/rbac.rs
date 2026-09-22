//! Predicate-based Role-Based Access Control (RBAC) helpers.
//!
//! Provides inline authorization functions for checking user claims and roles inside request handlers.

use crate::{error::AuthError, extractor::AuthUser};
use scyph_core::traits::Claims;

/// Validates that an authenticated user's claims satisfy a custom role predicate function.
///
/// Returns `Ok(())` if `predicate(&user.claims)` evaluates to `true`, or [`AuthError::Forbidden`] if `false`.
///
/// # Arguments
///
/// * `user` - Reference to the authenticated user [`AuthUser`].
/// * `predicate` - A closure or function evaluating claims (`&C -> bool`).
///
/// # Errors
///
/// Returns [`AuthError::Forbidden`] with message `"Insufficient role"` if the predicate evaluates to `false`.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::{AuthUser, require_role};
/// use scyph_core::Claims;
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// enum Role { Admin, User }
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct MyClaims { sub: Uuid, exp: i64, jti: String, role: Role }
/// impl Claims for MyClaims {
///     type Role = Role;
///     fn subject(&self) -> Uuid { self.sub }
///     fn expiry(&self) -> i64 { self.exp }
///     fn jti(&self) -> &str { &self.jti }
///     fn role(&self) -> &Self::Role { &self.role }
/// }
///
/// let user = AuthUser {
///     id: Uuid::new_v4(),
///     claims: MyClaims {
///         sub: Uuid::new_v4(),
///         exp: 10000,
///         jti: "jti".into(),
///         role: Role::Admin,
///     },
/// };
///
/// // Verify user is an admin
/// assert!(require_role(&user, |c| *c.role() == Role::Admin).is_ok());
/// assert!(require_role(&user, |c| *c.role() == Role::User).is_err());
/// ```
pub fn require_role<C: Claims, F>(user: &AuthUser<C>, predicate: F) -> Result<(), AuthError>
where
    F: Fn(&C) -> bool,
{
    if predicate(&user.claims) {
        Ok(())
    } else {
        Err(AuthError::Forbidden("Insufficient role".into()))
    }
}
