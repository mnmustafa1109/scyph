//! JSON Web Token (JWT) encoding, decoding, and validation.
//!
//! High-level wrapper around the [`jsonwebtoken`] library using secure types from [`secrecy`].

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
}

/// Encodes and signs a claims struct into a compact JWT string.
///
/// Uses standard HMAC-SHA256 (HS256) signing with the provided secret key.
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
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.expose_secret().as_bytes()),
    )
    .map_err(JwtError::Encoding)
}

/// Decodes and verifies a compact JWT string into typed claims [`TokenData<C>`].
///
/// Validates the token signature and verifies that the token expiration (`exp`) timestamp has not passed.
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
    let validation = Validation {
        validate_exp: true,
        ..Validation::default()
    };
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
