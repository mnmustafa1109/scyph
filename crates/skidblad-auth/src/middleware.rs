use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
};
use skidblad_core::{error::AppError, traits::Claims};

/// Zero-allocation wrapper holding a borrowed slice of permitted roles.
#[derive(Clone, Copy)]
pub struct AllowedRoles<R: 'static>(pub &'static [R]);

/// Checks if the authenticated user has one of the allowed roles.
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

    // 2. Fix Bug 1: pass reference to `contains`
    if allowed.0.contains(&user_role) {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Forbidden("Insufficient permissions".into()))
    }
}
