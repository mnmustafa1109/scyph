//! Password error types.

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
