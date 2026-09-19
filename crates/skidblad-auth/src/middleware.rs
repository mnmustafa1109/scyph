//! Role-based access control (RBAC) middleware for Axum routes.
//!
//! Evaluates authenticated user credentials extracted upstream and restricts access to routes
//! based on allowed roles.

use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
};
use skidblad_core::{error::AppError, traits::Claims};

/// Container struct holding a static slice of permitted roles for route access.
///
/// Used as an Axum [`Extension`] to configure role requirements on routes.
#[derive(Clone, Copy)]
pub struct AllowedRoles<R: 'static>(pub &'static [R]);

/// Middleware function enforcing role-based authorization rules.
///
/// Inspects the request extensions for the user's role (inserted upstream by an authentication
/// extractor or middleware) and compares it against [`AllowedRoles`].
///
/// # Errors
///
/// - Returns [`AppError::Unauthorized`] if no user role is present in request extensions.
/// - Returns [`AppError::Forbidden`] if the user's role is not in the list of allowed roles.
pub async fn require_roles_layer<C: Claims>(
    Extension(allowed): Extension<AllowedRoles<C::Role>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Retrieve the user that was authenticated upstream
    let user_role = match request.extensions().get::<C::Role>() {
        Some(role) => *role,
        None => {
            return Err(AppError::Unauthorized(
                "Unauthenticated: missing user identity".into(),
            ));
        }
    };

    // 2. Pass reference to `contains`
    if allowed.0.contains(&user_role) {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Forbidden("Insufficient permissions".into()))
    }
}
