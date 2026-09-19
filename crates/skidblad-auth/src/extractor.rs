// crates/skidblad-auth/src/extractor.rs

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use secrecy::SecretString;
use skidblad_core::{error::AppError, traits::Claims};
use std::sync::Arc;
use uuid::Uuid;

use crate::{cache::AuthCacheService, jwt::verify_token};

#[derive(Debug, Clone)]
pub struct AuthUser<C: Claims> {
    pub id: Uuid,
    pub claims: C,
}

pub trait AuthExtractorState<C: Claims>: Send + Sync + 'static {
    type CachePool: Clone + Send + Sync + 'static;

    fn jwt_secret(&self) -> &SecretString;
    fn auth_cache(&self) -> &Arc<AuthCacheService<Self::CachePool>>;
}

impl<S, C> FromRequestParts<S> for AuthUser<C>
where
    S: AuthExtractorState<C> + Send + Sync,
    C: Claims,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        // Fast-path: if a parent middleware or extractor already verified the user, return it
        if let Some(user) = parts.extensions.get::<AuthUser<C>>() {
            return Ok(user.clone());
        }

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Invalid Authorization scheme; expected Bearer".into())
        })?;

        let data = verify_token::<C>(token, state.jwt_secret())
            .map_err(|e| AppError::Unauthorized(e.to_string()))?;

        let claims = data.claims;
        let jti = claims.jti();

        if !jti.is_empty() && state.auth_cache().is_token_revoked(jti).await {
            return Err(AppError::Unauthorized("Token has been revoked".into()));
        }

        let user = AuthUser {
            id: claims.subject(),
            claims,
        };

        // Cache in request extensions for subsequent middleware or route handlers
        parts.extensions.insert(user.clone());

        Ok(user)
    }
}
