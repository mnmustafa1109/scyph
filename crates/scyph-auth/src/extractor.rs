//! Axum request extractors for authenticated users.
//!
//! Provides [`AuthUser<C>`], an extractor that parses, verifies, and validates JWT Bearer tokens
//! from incoming HTTP `Authorization` headers, checks token revocation, and injects user identity into handler parameters.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use secrecy::SecretString;
use scyph_core::{error::AppError, traits::Claims};
use std::sync::Arc;
use uuid::Uuid;

use crate::{cache::AuthCacheService, jwt::verify_token};

/// Authenticated user extracted from an HTTP request.
///
/// Contains the user's UUID `id` and parsed JWT `claims`.
///
/// # Type Parameters
/// - `C`: Your application's custom JWT claims struct implementing [`Claims`].
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_auth::AuthUser;
/// use scyph_core::Claims;
///
/// async fn handler<C: Claims>(user: AuthUser<C>) -> String {
///     format!("Hello, user {}", user.id)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser<C: Claims> {
    /// Unique subject identifier (UUID) extracted from the JWT `sub` claim.
    pub id: Uuid,
    /// Parsed JWT claims struct.
    pub claims: C,
}

/// Interface that application state (`S`) must implement to enable [`AuthUser`] extraction.
///
/// Your Axum application state must provide access to the JWT signing secret and the
/// authentication cache service.
pub trait AuthExtractorState<C: Claims>: Send + Sync + 'static {
    /// Associated cache profile type stored in [`AuthCacheService`].
    type CachePool: Clone + Send + Sync + 'static;

    /// Returns a reference to the secret key used for verifying JWT signatures.
    fn jwt_secret(&self) -> &SecretString;

    /// Returns a reference to the authentication cache service used for checking token revocation.
    fn auth_cache(&self) -> &Arc<AuthCacheService<Self::CachePool>>;
}

impl<S, C> FromRequestParts<S> for AuthUser<C>
where
    S: AuthExtractorState<C> + Send + Sync,
    C: Claims,
{
    type Rejection = AppError;

    /// Extracts [`AuthUser`] from request headers and extensions.
    ///
    /// # Extraction Workflow
    /// 1. **Fast Path**: Checks if [`AuthUser`] is already present in request extensions (e.g. added by middleware).
    /// 2. **Header Parsing**: Extracts the `Authorization` header and ensures it uses the `Bearer` scheme.
    /// 3. **Token Verification**: Validates the JWT signature and expiration using [`verify_token`].
    /// 4. **Revocation Check**: Queries [`AuthCacheService`] to verify the token's `jti` is not blacklisted.
    /// 5. **Caching**: Inserts the parsed [`AuthUser`] and [`Claims::Role`] into request extensions for downstream handlers.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        // Fast-path: if already verified, ensure both user and role are available
        if let Some(user) = parts.extensions.get::<AuthUser<C>>() {
            let role = *user.claims.role();
            let user = user.clone();

            if parts.extensions.get::<C::Role>().is_none() {
                parts.extensions.insert(role);
            }
            return Ok(user);
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
        let role = *claims.role();

        if !jti.is_empty() && state.auth_cache().is_token_revoked(jti).await {
            return Err(AppError::Unauthorized("Token has been revoked".into()));
        }

        let user = AuthUser {
            id: claims.subject(),
            claims,
        };

        // Cache role and user in request extensions
        parts.extensions.insert(role);
        parts.extensions.insert(user.clone());

        Ok(user)
    }
}
