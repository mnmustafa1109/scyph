//! Router extension traits for declarative role protection in Axum.
//!
//! Provides [`RoleRouterExt`], which adds `.require_roles(...)` to Axum [`Router`](axum::Router).

use axum::{Router, extract::Extension, middleware::from_fn_with_state};
use scyph_core::traits::Claims;

use crate::{
    extractor::AuthExtractorState,
    middleware::{AllowedRoles, require_roles_layer},
};

/// Fluent extension trait adding role-based route protection methods to Axum [`Router`].
pub trait RoleRouterExt<S> {
    /// Protects all routes currently configured in this [`Router`] instance with role requirements.
    ///
    /// Applies role validation middleware checking whether the authenticated user possesses one of the
    /// specified static `roles`.
    ///
    /// # Important Usage Note
    ///
    /// Like standard Axum layers, `.require_roles(...)` applies **only to routes configured prior to this call**.
    /// Ensure all target routes are defined on the [`Router`] before calling `.require_roles(...)`.
    ///
    /// # Arguments
    ///
    /// * `state` - The Axum application state implementing [`AuthExtractorState<C>`].
    /// * `roles` - A `'static` slice of allowed roles. Users must have one of these roles to access the routes.
    ///
    /// # Errors
    ///
    /// At request time, protected routes will return:
    /// - `401 Unauthorized` if the Bearer token is missing, expired, invalid, or revoked.
    /// - `403 Forbidden` if the user is authenticated but their role is not in `roles`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use axum::{Router, routing::get};
    /// use scyph_auth::RoleRouterExt;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    /// enum UserRole { User, Moderator, Admin }
    ///
    /// // Define routes first, then call require_roles —
    /// // only routes defined BEFORE this call are protected.
    ///
    /// // Admin-only routes (403 for User and Moderator)
    /// // let admin_routes = Router::new()
    /// //     .route("/admin/dashboard", get(dashboard))
    /// //     .route("/admin/users",     get(list_users))
    /// //     .require_roles::<AppClaims>(state.clone(), &[UserRole::Admin]);
    ///
    /// // Moderator and Admin routes
    /// // let mod_routes = Router::new()
    /// //     .route("/mod/reports", get(list_reports))
    /// //     .require_roles::<AppClaims>(state.clone(), &[UserRole::Moderator, UserRole::Admin]);
    ///
    /// // Merge into the main app
    /// // let app = Router::new()
    /// //     .merge(admin_routes)
    /// //     .merge(mod_routes)
    /// //     .route("/health", get(health_check)); // public, no auth required
    /// ```
    fn require_roles<C>(self, state: S, roles: &'static [C::Role]) -> Self
    where
        S: AuthExtractorState<C> + Clone + Send + Sync + 'static,
        C: Claims;
}

impl<S> RoleRouterExt<S> for Router<S> {
    fn require_roles<C>(self, state: S, roles: &'static [C::Role]) -> Self
    where
        S: AuthExtractorState<C> + Clone + Send + Sync + 'static,
        C: Claims,
    {
        self.layer(Extension(AllowedRoles(roles)))
            .route_layer(from_fn_with_state(state, require_roles_layer::<S, C>))
    }
}
