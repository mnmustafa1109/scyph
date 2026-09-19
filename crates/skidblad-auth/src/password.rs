// /crates/skidblad-auth/src/password.rs
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use secrecy::{ExposeSecret, SecretString};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("Hash failed: {0}")]
    HashFailed(String),

    #[error("Password verification failed")]
    VerificationFailed,

    #[error("Invalid hash format: {0}")]
    InvalidHash(String),
}

/// Hash a password using Argon2id with a random salt. Returns PHC string.
pub fn hash_password(pw: &SecretString) -> Result<String, PasswordError> {
    Argon2::default()
        .hash_password(pw.expose_secret().as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::HashFailed(e.to_string()))
}

/// Verify a plaintext password against an Argon2 PHC hash.
pub fn verify_password(pw: &SecretString, hash: &str) -> Result<(), PasswordError> {
    let parsed = PasswordHash::new(hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;
    Argon2::default()
        .verify_password(pw.expose_secret().as_bytes(), &parsed)
        .map_err(|_| PasswordError::VerificationFailed)
}
