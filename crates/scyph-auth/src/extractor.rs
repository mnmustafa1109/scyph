//! Axum request extractors for authenticated users.
//!
//! Provides [`AuthUser<C>`], an extractor that parses, verifies, and validates JWT Bearer tokens
//! from incoming HTTP `Authorization` headers, checks token revocation, and injects user identity into handler parameters.
//!
//! # Extraction Workflow
//!
//! When Axum resolves `AuthUser<C>` as a handler parameter, the following steps run in order:
//!
//! ```text
//! 1. Fast-path check
//!    └─ Is AuthUser<C> already in request extensions?
//!       YES → return it immediately (middleware pre-populated it)
//!       NO  → continue to step 2
//!
//! 2. Authorization header extraction
//!    └─ Read the `Authorization` HTTP header
//!       Missing → 401 Unauthorized ("Missing Authorization header")
//!
//! 3. Bearer scheme validation
//!    └─ Strip "Bearer " prefix from header value
//!       Wrong scheme → 401 Unauthorized ("Invalid Authorization scheme; expected Bearer")
//!
//! 4. JWT verification
//!    └─ verify_token::<C>(token, jwt_secret)
//!       Expired      → 401 Unauthorized ("Token has expired")
//!       Invalid sig  → 401 Unauthorized ("Invalid token: ...")
//!
//! 5. jti presence check
//!    └─ claims.jti() must be non-empty
//!       Empty jti    → 401 Unauthorized ("Token jti claim is missing")
//!
//! 6. Revocation check
//!    └─ AuthCacheService::is_token_revoked(jti)
//!       Revoked      → 401 Unauthorized ("Token has been revoked")
//!
//! 7. Success
//!    └─ AuthUser { id: claims.subject(), claims } inserted into extensions
//!       C::Role inserted into extensions for downstream middleware
//!       AuthUser returned to handler
//! ```
//!
//! # State Requirements
//!
//! Your Axum application state must implement [`AuthExtractorState<C>`] to provide:
//! - The JWT signing secret ([`AuthExtractorState::jwt_secret`])
//! - The authentication cache ([`AuthExtractorState::auth_cache`])
//!
//! # Usage in Handlers
//!
//! ```rust,no_run
//! use scyph_auth::{AuthUser, OptionalAuthUser};
//! use scyph_core::Claims;
//!
//! // Require authentication — returns 401 if token is absent or invalid
//! async fn protected_handler<C: Claims>(user: AuthUser<C>) -> String {
//!     format!("Welcome, user {}", user.id)
//! }
//!
//! // Optional authentication — never rejects the request
//! async fn hybrid_handler<C: Claims>(user: OptionalAuthUser<C>) -> String {
//!     match user.0 {
//!         Some(u) => format!("Hello, authenticated user {}", u.id),
//!         None    => "Hello, anonymous visitor!".into(),
//!     }
//! }
//! ```

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use scyph_core::{error::AppError, traits::Claims};
use secrecy::SecretString;
use std::sync::Arc;
use uuid::Uuid;

use crate::{cache::AuthCacheService, error::AuthError, jwt::verify_token};

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

/// Optional authenticated user extractor for public or hybrid endpoints.
///
/// Wraps an `Option<AuthUser<C>>`. If a valid `Authorization` header is present,
/// evaluates to `Some(AuthUser)`. If missing or invalid, evaluates to `None` without rejecting the HTTP request.
///
/// # Type Parameters
///
/// * `C` - Application claims type implementing [`Claims`].
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_auth::OptionalAuthUser;
/// use scyph_core::Claims;
///
/// async fn public_or_private_handler<C: Claims>(user: OptionalAuthUser<C>) -> String {
///     match user.0 {
///         Some(user) => format!("Hello authenticated user {}", user.id),
///         None => "Hello guest visitor".into(),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalAuthUser<C: Claims>(pub Option<AuthUser<C>>);

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

use tracing::{debug, warn};

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
    ///    If found, the role is also re-inserted if missing, and the cached user is returned immediately.
    /// 2. **Header Parsing**: Extracts the `Authorization` header and ensures it uses the `Bearer` scheme.
    ///    Returns `401 Unauthorized` if the header is absent or uses a different scheme.
    /// 3. **Token Verification**: Calls [`verify_token`] to validate the JWT signature and expiration.
    ///    Returns `401 Unauthorized` with [`JwtError`](crate::jwt::JwtError) detail if invalid.
    /// 4. **jti Validation**: Ensures the `jti` claim is non-empty (required for revocation support).
    /// 5. **Revocation Check**: Queries [`AuthCacheService`] to verify the token's `jti` is not blacklisted.
    ///    Returns `401 Unauthorized` if the token has been explicitly revoked.
    /// 6. **Extension Population**: Inserts the resolved [`AuthUser`] and `C::Role` into request extensions
    ///    so downstream middleware and handlers can access them cheaply without re-parsing.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Unauthorized`] (HTTP 401) on any authentication failure.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        // Fast-path: if already verified, ensure both user and role are available
        if let Some(user) = parts.extensions.get::<AuthUser<C>>() {
            let role = *user.claims.role();
            let user = user.clone();

            if parts.extensions.get::<C::Role>().is_none() {
                parts.extensions.insert(role);
            }
            debug!(user_id = %user.id, "AuthUser retrieved from request extensions (fast-path)");
            return Ok(user);
        }

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                warn!("Authentication failed: Missing Authorization header");
                AuthError::Unauthorized("Missing Authorization header".into())
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            warn!("Authentication failed: Invalid Authorization scheme (expected Bearer)");
            AuthError::Unauthorized("Invalid Authorization scheme; expected Bearer".into())
        })?;

        let data = verify_token::<C>(token, state.jwt_secret()).map_err(|e| {
            warn!(error = %e, "Authentication failed: JWT verification failed");
            AuthError::Unauthorized(e.to_string())
        })?;

        let claims = data.claims;
        let jti = claims.jti();
        let role = *claims.role();

        if jti.is_empty() {
            warn!("Authentication failed: Token jti claim is missing");
            return Err(AuthError::Unauthorized("Token jti claim is missing".into()).into());
        }

        if state.auth_cache().is_token_revoked(jti).await {
            warn!(jti = %jti, "Authentication failed: Token has been revoked");
            return Err(AuthError::Unauthorized("Token has been revoked".into()).into());
        }

        let user = AuthUser {
            id: claims.subject(),
            claims,
        };

        debug!(user_id = %user.id, "Successfully authenticated user via Bearer JWT");

        // Cache role and user in request extensions
        parts.extensions.insert(role);
        parts.extensions.insert(user.clone());

        Ok(user)
    }
}

impl<S, C> FromRequestParts<S> for OptionalAuthUser<C>
where
    S: AuthExtractorState<C> + Send + Sync,
    C: Claims,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            AuthUser::<C>::from_request_parts(parts, state).await.ok(),
        ))
    }
}
