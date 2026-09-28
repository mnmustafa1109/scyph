//! Role-based access control (RBAC) middleware for Axum routes.
//!
//! Evaluates authenticated user credentials extracted upstream and restricts access to routes
//! based on allowed roles.
//!
//! # How It Works
//!
//! [`require_roles_layer`] is an Axum middleware function that:
//! 1. Extracts and verifies the JWT token from the request (delegates to [`AuthUser`]).
//! 2. Reads the [`AllowedRoles`] extension injected by [`RoleRouterExt::require_roles`].
//! 3. Checks whether the authenticated user's role appears in the allowed list.
//! 4. On success, reconstructs the request (with `AuthUser` in extensions) and calls `next`.
//!
//! # Integration
//!
//! This middleware is not typically used directly. The idiomatic API is via [`RoleRouterExt`]:
//!
//! ```rust,no_run
//! use axum::{Router, routing::get};
//! use scyph_auth::RoleRouterExt;
//!
//! // (Assuming AppClaims and UserRole are defined in your app)
//! // let protected = Router::new()
//! //     .route("/admin", get(admin_handler))
//! //     .require_roles::<AppClaims>(state, &[UserRole::Admin, UserRole::Moderator]);
//! ```
//!
//! If you need to apply the middleware manually (e.g., for custom layer ordering), use:
//!
//! ```rust,no_run
//! use axum::{Router, routing::get, extract::Extension, middleware::from_fn_with_state};
//! use scyph_auth::middleware::{AllowedRoles, require_roles_layer};
//!
//! // let router = Router::new()
//! //     .route("/admin", get(admin_handler))
//! //     .layer(Extension(AllowedRoles(&[UserRole::Admin])))
//! //     .route_layer(from_fn_with_state(state, require_roles_layer::<AppState, AppClaims>));
//! ```

use crate::{AuthExtractorState, AuthUser, error::AuthError};
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

use tracing::{debug, warn};

/// Combined authentication and role authorization middleware layer.
///
/// Extracts and verifies the user's JWT credentials, checks token revocation, and enforces
/// that the authenticated user's role is in [`AllowedRoles`].
///
/// # Arguments
///
/// * `state` - The Axum application state, must implement [`AuthExtractorState<C>`].
/// * `allowed` - The [`AllowedRoles`] extension containing the permitted roles for this route.
/// * `request` - The incoming HTTP request.
/// * `next` - The next middleware or handler in the Axum tower stack.
///
/// # Errors
///
/// - Returns [`AppError::Unauthorized`] (HTTP 401) if the token is missing, invalid, expired, or revoked.
///   This is propagated from [`AuthUser::from_request_parts`].
/// - Returns [`AppError::Forbidden`] (HTTP 403) if the user is authenticated but their role
///   does not appear in [`AllowedRoles`].
///
/// # Notes
///
/// - The `AuthUser` is inserted into request extensions after successful verification, so
///   downstream handlers receiving `AuthUser<C>` as a parameter will use the fast-path and
///   avoid redundant token verification.
/// - Role comparison uses `PartialEq` via `AllowedRoles::0.contains()`, which requires
///   `C::Role` to implement [`Eq`] (guaranteed by [`Authorizable`](scyph_core::traits::Authorizable)).
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

    // 4. Reconstruct request with parts (containing AuthUser extensions) and body
    let request = Request::from_parts(parts, body);

    Ok(next.run(request).await)
}
