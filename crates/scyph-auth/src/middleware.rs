//! Role-based access control (RBAC) middleware for Axum routes.
//!
//! Evaluates authenticated user credentials extracted upstream and restricts access to routes
//! based on allowed roles.

use crate::{AuthExtractorState, AuthUser};
use axum::{
    extract::{Extension, FromRequestParts, Request, State},
    middleware::Next,
    response::Response,
};
use scyph_core::{error::AppError, traits::Claims};

/// Container struct holding a static slice of permitted roles for route access.
///
/// Used as an Axum [`Extension`] to configure role requirements on routes.
#[derive(Clone, Copy)]
pub struct AllowedRoles<R: 'static>(pub &'static [R]);

/// Combined authentication and role authorization middleware layer.
///
/// Extracts and verifies the user's JWT credentials, checks token revocation, and enforces
/// that the authenticated user's role is in [`AllowedRoles`].
///
/// # Errors
///
/// - Returns [`AppError::Unauthorized`] if the token is missing, invalid, expired, or revoked.
/// - Returns [`AppError::Forbidden`] if the user's role is not in the list of allowed roles.
pub async fn require_roles_layer<S, C>(
    State(state): State<S>,
    Extension(allowed): Extension<AllowedRoles<C::Role>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError>
where
    S: AuthExtractorState<C> + Send + Sync,
    C: Claims,
{
    // 1. Destructure request into HTTP parts and body
    let (mut parts, body) = request.into_parts();

    // 2. Extract AuthUser (verifies JWT, checks revocation, populates extensions)
    let user = AuthUser::<C>::from_request_parts(&mut parts, &state).await?;

    // 3. Verify user's role against allowed list
    if !allowed.0.contains(user.claims.role()) {
        return Err(AppError::Forbidden("Insufficient permissions".into()));
    }

    // 4. Reconstruct request with parts (containing AuthUser extensions) and body
    let request = Request::from_parts(parts, body);

    Ok(next.run(request).await)
}
