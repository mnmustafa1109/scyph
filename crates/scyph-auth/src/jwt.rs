//! JSON Web Token (JWT) encoding, decoding, and validation.
//!
//! High-level wrapper around the [`jsonwebtoken`] library using secure types from [`secrecy`].
//!
//! # Algorithm
//!
//! All tokens are signed using **HMAC-SHA256 (HS256)** — a symmetric algorithm where the same
//! secret is used for both signing and verification. This is appropriate for single-service
//! or service-mesh deployments where the secret can be shared securely. For multi-party
//! scenarios requiring asymmetric signing (RS256/ES256), extend or replace these functions.
//!
//! # Security Considerations
//!
//! - Read the signing secret with [`jwt_secret_from_env()`]: it refuses `JWT_SECRET` values
//!   shorter than 32 bytes (256 bits), so a brute-forceable key is rejected at boot instead
//!   of silently signing forgeable tokens.
//! - The `secret` parameter is wrapped in [`secrecy::SecretString`] to prevent it from
//!   accidentally appearing in log output, debug formatting, or error messages.
//! - Tokens include a `jti` (JWT ID) claim. Use [`AuthCacheService::blacklist_token`] to
//!   revoke individual tokens before their natural expiration.
//! - Set `JWT_ISSUER` to stamp every token with an `iss` claim and require a matching
//!   `iss` on verification. Unset (the default) keeps tokens issuer-free and
//!   verification issuer-agnostic, preserving the pre-`JWT_ISSUER` contract.
//! - Set `JWT_EXPIRATION` (seconds) to make token lifetime configurable per
//!   environment: [`jwt_expiration_from_env()`] turns it (default 24h) into the
//!   offset your claims builder adds to "now" for the `exp` claim.
//! - Always set a short expiration (`exp`) on access tokens (e.g., 15–60 minutes).
//!   Use a separate refresh token flow for long-lived sessions.
//!
//! # Examples
//!
//! Full round-trip: create a token, verify it, and read claims back:
//!
//! ```rust
//! use scyph_auth::{create_token, verify_token};
//! use scyph_core::Claims;
//! use secrecy::SecretString;
//! use serde::{Serialize, Deserialize};
//! use uuid::Uuid;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
//! enum Role { Admin, User }
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! struct AppClaims {
//!     sub: Uuid,
//!     exp: i64,
//!     jti: String,
//!     role: Role,
//!     email: String,
//! }
//!
//! impl Claims for AppClaims {
//!     type Role = Role;
//!     fn subject(&self) -> Uuid     { self.sub }
//!     fn expiry(&self) -> i64       { self.exp }
//!     fn jti(&self) -> &str         { &self.jti }
//!     fn role(&self) -> &Self::Role { &self.role }
//! }
//!
//! let user_id = Uuid::now_v7();
//! let claims = AppClaims {
//!     sub: user_id,
//!     exp: chrono::Utc::now().timestamp() + 3600,
//!     jti: Uuid::now_v7().to_string(),
//!     role: Role::Admin,
//!     email: "alice@example.com".into(),
//! };
//!
//! let secret = SecretString::from("at-least-32-bytes-long-secret-key!!");
//!
//! // Sign the token
//! let token = create_token(&claims, &secret).expect("encoding failed");
//! assert!(!token.is_empty());
//!
//! // Verify and decode
//! let token_data = verify_token::<AppClaims>(&token, &secret).expect("verification failed");
//! assert_eq!(token_data.claims.subject(), user_id);
//! assert_eq!(*token_data.claims.role(), Role::Admin);
//! assert_eq!(token_data.claims.email, "alice@example.com");
//! ```

use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use scyph_core::traits::Claims;
use secrecy::{ExposeSecret, SecretString};

/// Errors returned during JWT creation, decoding, or signature verification.
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    /// Failed to encode claims into a signed JWT string.
    #[error("Token encoding failed: {0}")]
    Encoding(#[from] jsonwebtoken::errors::Error),

    /// The JWT signature is valid, but the token expiration (`exp`) has passed.
    #[error("Token has expired")]
    Expired,

    /// The token is malformed, invalid, or has an invalid signature.
    #[error("Invalid token: {0}")]
    Invalid(String),

    /// The signing secret is missing from the environment or fails the minimum-length rule.
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Minimum length of an HS256 signing secret — 256 bits of entropy.
const MIN_JWT_SECRET_LEN: usize = 32;

/// Reads the HS256 signing secret from the `JWT_SECRET` environment variable.
///
/// Refuses weak configuration at boot: a secret shorter than 256 bits is
/// brute-forceable, which would let an attacker forge tokens. Both a missing
/// variable and a too-short secret return [`JwtError::Config`] so applications
/// can fail fast instead of signing forgeable tokens.
///
/// # Errors
///
/// Returns [`JwtError::Config`] if `JWT_SECRET` is unset or shorter than 32 bytes.
pub fn jwt_secret_from_env() -> Result<SecretString, JwtError> {
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| JwtError::Config("JWT_SECRET environment variable must be set".into()))?;
    validate_jwt_secret(&secret)?;
    Ok(SecretString::from(secret))
}

/// Enforces the minimum HS256 secret length (bytes, not characters).
fn validate_jwt_secret(secret: &str) -> Result<(), JwtError> {
    if secret.len() < MIN_JWT_SECRET_LEN {
        return Err(JwtError::Config(format!(
            "JWT_SECRET must be at least {MIN_JWT_SECRET_LEN} bytes (256 bits of entropy)"
        )));
    }
    Ok(())
}

/// Optional issuer from `JWT_ISSUER`: when set, [`create_token`] stamps the
/// `iss` claim and [`verify_token`] requires a matching value.
fn jwt_issuer_from_env() -> Option<String> {
    std::env::var("JWT_ISSUER")
        .ok()
        .map(|iss| iss.trim().to_string())
        .filter(|iss| !iss.is_empty())
}

/// Returns the default token validity in **seconds** for building `exp` claims,
/// read from the optional `JWT_EXPIRATION` environment variable (default: 86_400 = 24 hours).
///
/// Keeps token lifetime configurable per environment instead of hardcoded at the
/// mint site — add it to the current Unix timestamp when constructing claims:
///
/// ```rust
/// use scyph_auth::jwt_expiration_from_env;
///
/// let lifetime = jwt_expiration_from_env().expect("JWT_EXPIRATION must parse");
/// let exp = chrono::Utc::now().timestamp() + lifetime;
/// # let _ = exp;
/// ```
///
/// # Errors
///
/// Returns [`JwtError::Config`] when `JWT_EXPIRATION` is set but not a positive integer.
pub fn jwt_expiration_from_env() -> Result<i64, JwtError> {
    const DEFAULT_SECS: i64 = 86_400;
    let raw = match std::env::var("JWT_EXPIRATION") {
        Ok(raw) => raw,
        Err(_) => return Ok(DEFAULT_SECS),
    };
    let secs: i64 = raw.trim().parse().map_err(|_| {
        JwtError::Config("JWT_EXPIRATION must be a positive number of seconds".into())
    })?;
    if secs <= 0 {
        return Err(JwtError::Config(
            "JWT_EXPIRATION must be a positive number of seconds".into(),
        ));
    }
    Ok(secs)
}

/// Encodes and signs a claims struct into a compact JWT string.
///
/// Uses standard HMAC-SHA256 (HS256) signing with the provided secret key.
/// When `JWT_ISSUER` is set, the token is stamped with that value as its `iss` claim.
///
/// # Arguments
///
/// * `claims` - Reference to your application's claims struct implementing [`Claims`].
/// * `secret` - Secret key used for signing the token.
///
/// # Errors
///
/// Returns [`JwtError::Encoding`] if serialization or signing fails.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::create_token;
/// use scyph_core::Claims;
/// use secrecy::SecretString;
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// enum Role { User }
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct MyClaims { sub: Uuid, exp: i64, jti: String, role: Role }
/// impl Claims for MyClaims {
///     type Role = Role;
///     fn subject(&self) -> Uuid { self.sub }
///     fn expiry(&self) -> i64 { self.exp }
///     fn jti(&self) -> &str { &self.jti }
///     fn role(&self) -> &Self::Role { &self.role }
/// }
///
/// let claims = MyClaims {
///     sub: Uuid::now_v7(),
///     exp: chrono::Utc::now().timestamp() + 3600,
///     jti: "unique_id".into(),
///     role: Role::User,
/// };
/// let secret = SecretString::from("super_secret_key");
/// let token = create_token(&claims, &secret).unwrap();
/// ```
pub fn create_token<C: Claims>(claims: &C, secret: &SecretString) -> Result<String, JwtError> {
    // JWT_ISSUER (optional) stamps the `iss` claim; verify_token requires the
    // same value, so signing and verification always agree within one process.
    match jwt_issuer_from_env() {
        None => encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.expose_secret().as_bytes()),
        )
        .map_err(JwtError::Encoding),
        Some(issuer) => {
            let mut payload = serde_json::to_value(claims)
                .map_err(jsonwebtoken::errors::Error::from)
                .map_err(JwtError::Encoding)?;
            payload["iss"] = serde_json::Value::String(issuer);
            encode(
                &Header::default(),
                &payload,
                &EncodingKey::from_secret(secret.expose_secret().as_bytes()),
            )
            .map_err(JwtError::Encoding)
        }
    }
}

/// Decodes and verifies a compact JWT string into typed claims [`TokenData<C>`].
///
/// Validates the token signature and verifies that the token expiration (`exp`) timestamp has not passed.
/// When `JWT_ISSUER` is set, the token must also carry a matching `iss` claim (tokens
/// issued before the variable was set are rejected and their holders must re-authenticate).
///
/// # Arguments
///
/// * `token` - The compact JWT string (e.g. from a `Bearer` header).
/// * `secret` - Secret key used to verify the HMAC signature.
///
/// # Errors
///
/// - Returns [`JwtError::Expired`] if the token has expired.
/// - Returns [`JwtError::Invalid`] if signature verification fails or the token is malformed.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::{create_token, verify_token};
/// use scyph_core::Claims;
/// use secrecy::SecretString;
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// enum Role { User }
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct MyClaims { sub: Uuid, exp: i64, jti: String, role: Role }
/// impl Claims for MyClaims {
///     type Role = Role;
///     fn subject(&self) -> Uuid { self.sub }
///     fn expiry(&self) -> i64 { self.exp }
///     fn jti(&self) -> &str { &self.jti }
///     fn role(&self) -> &Self::Role { &self.role }
/// }
///
/// let claims = MyClaims {
///     sub: Uuid::now_v7(),
///     exp: chrono::Utc::now().timestamp() + 3600,
///     jti: "token_123".into(),
///     role: Role::User,
/// };
/// let secret = SecretString::from("secret_key");
/// let token = create_token(&claims, &secret).unwrap();
///
/// let token_data = verify_token::<MyClaims>(&token, &secret).unwrap();
/// assert_eq!(token_data.claims.jti(), "token_123");
/// ```
pub fn verify_token<C: Claims>(
    token: &str,
    secret: &SecretString,
) -> Result<TokenData<C>, JwtError> {
    let mut validation = Validation {
        validate_exp: true,
        ..Validation::default()
    };
    // JWT_ISSUER (optional): when set, tokens must carry a matching `iss` claim.
    if let Some(issuer) = jwt_issuer_from_env() {
        validation.set_issuer(&[issuer]);
        validation.required_spec_claims.insert("iss".into());
    }
    decode::<C>(
        token,
        &DecodingKey::from_secret(secret.expose_secret().as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
        _ => JwtError::Invalid(e.to_string()),
    })
}
