//! Password hashing and verification error types.
//!
//! Defines [`PasswordError`] — failures from the Argon2id password hashing and
//! constant-time verification pipeline. These are produced by [`crate::hash_password`],
//! [`crate::verify_password`], and their `_async` variants.

use thiserror::Error;

/// Errors encountered during Argon2id password hashing or verification.
///
/// ## HTTP Mapping
///
/// All `PasswordError` variants convert to HTTP 401 Unauthorized when wrapped in
/// [`crate::AuthError::Password`], because a password failure indicates invalid credentials
/// rather than a server fault.
///
/// ## Security Note
///
/// `VerificationFailed` is intentionally opaque — it does not distinguish between "wrong password"
/// and "user not found" to prevent username enumeration attacks. Only the specific hash validation
/// failure is hidden; all other error details are surfaced for debugging.
///
/// # Examples
///
/// ```rust
/// use scyph_auth::{hash_password, verify_password, PasswordError};
/// use secrecy::SecretString;
///
/// let password = SecretString::from("correct_password");
/// let wrong = SecretString::from("wrong_password");
/// let hash = hash_password(&password).unwrap();
///
/// // Correct password succeeds
/// assert!(verify_password(&password, &hash).is_ok());
///
/// // Wrong password returns VerificationFailed
/// let err = verify_password(&wrong, &hash).unwrap_err();
/// assert!(matches!(err, PasswordError::VerificationFailed));
///
/// // Malformed hash returns InvalidHash
/// let err = verify_password(&password, "not_a_valid_hash").unwrap_err();
/// assert!(matches!(err, PasswordError::InvalidHash(_)));
/// ```
#[derive(Debug, Error)]
pub enum PasswordError {
    /// Argon2id salt generation or hash computation failed.
    ///
    /// This is typically a system-level failure (e.g., entropy source unavailable).
    #[error("Hash failed: {0}")]
    HashFailed(String),

    /// Plaintext password does not match the stored Argon2id PHC hash.
    ///
    /// Returned by constant-time comparison to prevent timing side-channel attacks.
    #[error("Password verification failed")]
    VerificationFailed,

    /// The provided hash string is not a valid PHC-formatted Argon2id hash.
    ///
    /// Typically caused by storing a plaintext password in the hash column, or data corruption.
    #[error("Invalid hash format: {0}")]
    InvalidHash(String),
}
