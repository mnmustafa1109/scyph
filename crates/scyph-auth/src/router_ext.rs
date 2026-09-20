//! Router extension traits for declarative role protection in Axum.
//!
//! Provides [`RoleRouterExt`], which adds `.require_roles(...)` to Axum [`Router`](axum::Router).

use axum::{Router, extract::Extension, middleware::from_fn};
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
    /// # Arguments
    ///
    /// * `roles` - Static slice of allowed roles.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use axum::{Router, routing::get};
    /// use scyph_auth::RoleRouterExt;
    ///
    /// #[derive(Clone, Copy, PartialEq, Eq)]
    /// enum UserRole { Admin, Moderator }
    ///
    /// // Protected admin router
    /// // let admin_routes = Router::new()
    /// //     .route("/dashboard", get(|| async { "Admin Dashboard" }))
    /// //     .require_roles::<MyClaims>(&[UserRole::Admin]);
    /// ```
    fn require_roles<C>(self, roles: &'static [C::Role]) -> Self
    where
        S: AuthExtractorState<C> + Clone + Send + Sync + 'static,
        C: Claims;
}

impl<S> RoleRouterExt<S> for Router<S> {
    fn require_roles<C>(self, roles: &'static [C::Role]) -> Self
    where
        S: AuthExtractorState<C> + Clone + Send + Sync + 'static,
        C: Claims,
    {
        self.layer(Extension(AllowedRoles(roles)))
            .route_layer(from_fn(require_roles_layer::<C>))
    }
}
