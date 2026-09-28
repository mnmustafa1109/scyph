//! Secure password hashing and constant-time verification using Argon2id.
//!
//! Integrates [`argon2`] and [`secrecy`] to handle user passwords safely without leaving secrets in memory.
//! Provides [`PasswordService`] for configurable, concurrency-managed password hashing in production backends.

mod error;
mod service;

pub use error::PasswordError;
pub use service::PasswordService;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use secrecy::{ExposeSecret, SecretString};
use std::sync::LazyLock;

static DEFAULT_SERVICE: LazyLock<PasswordService> = LazyLock::new(PasswordService::default);

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
/// Delegates to a default [`PasswordService`] instance.
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
    DEFAULT_SERVICE.hash_password(pw).await
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
/// Delegates to [`verify_password`] inside Tokio's blocking task pool.
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
pub async fn verify_password_async(
    pw: &SecretString,
    hash: impl AsRef<str>,
) -> Result<(), PasswordError> {
    DEFAULT_SERVICE.verify_password(pw, hash.as_ref()).await
}
