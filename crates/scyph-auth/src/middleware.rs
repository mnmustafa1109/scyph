//! Role-based access control (RBAC) middleware for Axum routes.
//!
//! Evaluates authenticated user credentials extracted upstream and restricts access to routes
//! based on allowed roles.

use crate::{AuthExtractorState, AuthUser, error::AuthError};
use axum::{
    extract::{FromRequestParts, Request, State},
    middleware::Next,
    response::Response,
};
use scyph_core::{error::AppError, traits::Claims};
use std::sync::Arc;
use tracing::{debug, warn};

/// Container struct holding permitted roles for route access.
///
/// Wraps an `Arc<[R]>` to avoid unnecessary allocations across requests while allowing
/// flexible initialization from static slices, vectors, or Arcs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowedRoles<R: 'static>(pub Arc<[R]>);

impl<R: Clone + 'static> AllowedRoles<R> {
    /// Constructs [`AllowedRoles`] from any type convertible into `Arc<[R]>`.
    pub fn new(roles: impl Into<Arc<[R]>>) -> Self {
        Self(roles.into())
    }

    /// Constructs [`AllowedRoles`] from a slice of roles.
    pub fn from_slice(roles: &[R]) -> Self {
        Self(Arc::from(roles))
    }

    /// Returns a slice view of the allowed roles.
    pub fn roles(&self) -> &[R] {
        &self.0
    }

    /// Checks if a role is permitted by this allowed list.
    pub fn contains(&self, role: &R) -> bool
    where
        R: PartialEq,
    {
        self.0.contains(role)
    }
}

impl<R: Clone + 'static> From<&'static [R]> for AllowedRoles<R> {
    fn from(slice: &'static [R]) -> Self {
        Self(Arc::from(slice))
    }
}

impl<R: 'static> From<Vec<R>> for AllowedRoles<R> {
    fn from(vec: Vec<R>) -> Self {
        Self(Arc::from(vec))
    }
}

impl<R: 'static> From<Arc<[R]>> for AllowedRoles<R> {
    fn from(arc: Arc<[R]>) -> Self {
        Self(arc)
    }
}

impl<R: 'static> std::ops::Deref for AllowedRoles<R> {
    type Target = [R];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// State passed to [`require_roles_layer`] combining application auth state with allowed roles.
#[derive(Clone, Debug)]
pub struct RoleAuthState<S, R: 'static> {
    /// Application auth state implementing [`AuthExtractorState`].
    pub auth_state: S,
    /// Permitted roles for the protected route.
    pub allowed_roles: AllowedRoles<R>,
}

impl<S, R: 'static> RoleAuthState<S, R> {
    /// Constructs a new [`RoleAuthState`] with the given state and allowed roles.
    pub fn new(auth_state: S, allowed_roles: impl Into<AllowedRoles<R>>) -> Self {
        Self {
            auth_state,
            allowed_roles: allowed_roles.into(),
        }
    }
}

/// Combined authentication and role authorization middleware layer.
///
/// Extracts and verifies the user's JWT credentials, checks token revocation, and enforces
/// that the authenticated user's role is in [`AllowedRoles`].
///
/// # Errors
///
/// - Returns [`AuthError::Unauthorized`] if the token is missing, invalid, expired, or revoked.
/// - Returns [`AuthError::Forbidden`] if the user's role is not in the list of allowed roles.
pub async fn require_roles_layer<S, C>(
    State(role_state): State<RoleAuthState<S, C::Role>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError>
where
    S: AuthExtractorState<C> + Clone + Send + Sync + 'static,
    C: Claims,
{
    // 1. Destructure request into HTTP parts and body
    let (mut parts, body) = request.into_parts();

    // 2. Extract AuthUser (verifies JWT, checks revocation, populates extensions)
    let user = AuthUser::<C>::from_request_parts(&mut parts, &role_state.auth_state).await?;

    // 3. Verify user's role against allowed list
    if !role_state.allowed_roles.contains(user.claims.role()) {
        warn!(
            user_id = %user.id,
            "RBAC Authorization failed: Insufficient role permissions"
        );
        return Err(AuthError::Forbidden("Insufficient permissions".into()).into());
    }

    debug!(
        user_id = %user.id,
        "RBAC Authorization passed for route"
    );

    // Make allowed roles available in extensions for downstream inspection if desired
    parts.extensions.insert(role_state.allowed_roles.clone());

    // 4. Reconstruct request with parts (containing AuthUser extensions) and body
    let request = Request::from_parts(parts, body);

    Ok(next.run(request).await)
}
