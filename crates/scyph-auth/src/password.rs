//! Secure password hashing and constant-time verification using Argon2id.
//!
//! Integrates [`argon2`] and [`secrecy`] to handle user passwords safely without leaving secrets in memory.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use secrecy::{ExposeSecret, SecretString};

/// Errors encountered during password hashing or verification.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    /// Failed to calculate password hash.
    #[error("Hash failed: {0}")]
    HashFailed(String),

    /// Plaintext password does not match stored hash.
    #[error("Password verification failed")]
    VerificationFailed,

    /// Stored password hash string is not a valid PHC formatted hash.
    #[error("Invalid hash format: {0}")]
    InvalidHash(String),
}

/// Hashes a plaintext password string using the Argon2id password hashing algorithm.
///
/// Returns a PHC-formatted hash string suitable for storing in a database.
///
/// # Arguments
///
/// * `pw` - Plaintext password wrapped in a [`SecretString`].
///
/// # Errors
///
/// Returns [`PasswordError::HashFailed`] if salt generation or Argon2 computation fails.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::hash_password;
/// use secrecy::SecretString;
///
/// let password = SecretString::from("my_secure_password_123");
/// let phc_hash = hash_password(&password).unwrap();
/// assert!(phc_hash.starts_with("$argon2id$"));
/// ```
pub fn hash_password(pw: &SecretString) -> Result<String, PasswordError> {
    Argon2::default()
        .hash_password(pw.expose_secret().as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::HashFailed(e.to_string()))
}

/// Hashes a plaintext password asynchronously on Tokio's blocking thread pool (`spawn_blocking`).
///
/// Prevents CPU-bound Argon2 computation from stalling Tokio's async worker threads under load.
///
/// # Arguments
///
/// * `pw` - Plaintext password wrapped in a [`SecretString`].
///
/// # Errors
///
/// Returns [`PasswordError::HashFailed`] if salt generation, Argon2 computation, or thread join fails.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::hash_password_async;
/// use secrecy::SecretString;
///
/// # #[tokio::main]
/// # async fn main() {
/// let password = SecretString::from("my_secure_password_123");
/// let phc_hash = hash_password_async(&password).await.unwrap();
/// assert!(phc_hash.starts_with("$argon2id$"));
/// # }
/// ```
pub async fn hash_password_async(pw: &SecretString) -> Result<String, PasswordError> {
    let password = pw.clone();
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| PasswordError::HashFailed(e.to_string()))?
}

/// Verifies a plaintext password against an Argon2 PHC formatted hash string in constant time.
///
/// # Arguments
///
/// * `pw` - Plaintext password wrapped in a [`SecretString`].
/// * `hash` - The Argon2 PHC formatted hash string to verify against.
///
/// # Errors
///
/// - Returns [`PasswordError::InvalidHash`] if `hash` is not a valid PHC string.
/// - Returns [`PasswordError::VerificationFailed`] if password verification fails.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::{hash_password, verify_password};
/// use secrecy::SecretString;
///
/// let password = SecretString::from("my_secret_password");
/// let phc_hash = hash_password(&password).unwrap();
///
/// assert!(verify_password(&password, &phc_hash).is_ok());
///
/// let wrong_password = SecretString::from("wrong_password");
/// assert!(verify_password(&wrong_password, &phc_hash).is_err());
/// ```
pub fn verify_password(pw: &SecretString, hash: &str) -> Result<(), PasswordError> {
    let parsed = PasswordHash::new(hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;
    Argon2::default()
        .verify_password(pw.expose_secret().as_bytes(), &parsed)
        .map_err(|_| PasswordError::VerificationFailed)
}

/// Verifies a plaintext password asynchronously on Tokio's blocking thread pool (`spawn_blocking`).
///
/// # Arguments
///
/// * `pw` - Plaintext password wrapped in a [`SecretString`].
/// * `hash` - The Argon2 PHC formatted hash string to verify against.
///
/// # Errors
///
/// - Returns [`PasswordError::InvalidHash`] if `hash` is not a valid PHC string.
/// - Returns [`PasswordError::VerificationFailed`] if password verification fails.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::{hash_password_async, verify_password_async};
/// use secrecy::SecretString;
///
/// # #[tokio::main]
/// # async fn main() {
/// let password = SecretString::from("my_secret_password");
/// let phc_hash = hash_password_async(&password).await.unwrap();
///
/// assert!(verify_password_async(&password, &phc_hash).await.is_ok());
/// # }
/// ```
pub async fn verify_password_async(pw: &SecretString, hash: impl Into<String>) -> Result<(), PasswordError> {
    let password = pw.clone();
    let hash = hash.into();
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| PasswordError::HashFailed(e.to_string()))?
}
