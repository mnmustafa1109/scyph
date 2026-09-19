// /crates/skidblad-auth/src/router_ext.rs

use axum::{Router, extract::Extension, middleware::from_fn};
use skidblad_core::traits::Claims;

use crate::{
    extractor::AuthExtractorState,
    middleware::{AllowedRoles, require_roles_layer},
};

pub trait RoleRouterExt<S> {
    /// Protects all routes currently in this Router instance.
    /// Runs authentication extraction followed by static role validation.
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
