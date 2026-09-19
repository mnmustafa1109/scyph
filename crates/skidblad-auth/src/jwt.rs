// crates/skidblad-auth/src/jwt.rs
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use secrecy::{ExposeSecret, SecretString};
use skidblad_core::traits::Claims;

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Token encoding failed: {0}")]
    Encoding(#[from] jsonwebtoken::errors::Error),

    #[error("Token has expired")]
    Expired,

    #[error("Invalid token: {0}")]
    Invalid(String),
}

/// Sign a claims struct into a compact JWT string.
pub fn create_token<C: Claims>(claims: &C, secret: &SecretString) -> Result<String, JwtError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.expose_secret().as_bytes()),
    )
    .map_err(JwtError::Encoding)
}

/// Verify a JWT string and return decoded claims. Validates signature and exp.
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
