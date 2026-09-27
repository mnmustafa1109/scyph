//! Password service for concurrency-managed password hashing and verification.

use std::sync::Arc;
use tokio::sync::Semaphore;
use secrecy::{ExposeSecret, SecretString};
use argon2::{
    Argon2, Params,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use crate::password::PasswordError;

/// Production-grade password hashing service with configurable concurrency and Argon2 parameters.
///
/// Encapsulates a shared [`Semaphore`] concurrency limiter and Argon2 cost parameters.
/// Designed for dependency injection in Axum application state (`State<AppState>`).
///
/// # Examples
///
/// ```rust
/// use scyph_auth::PasswordService;
/// use secrecy::SecretString;
///
/// # #[tokio::main]
/// # async fn main() {
/// let service = PasswordService::default();
/// let password = SecretString::from("my_secure_password_123");
/// let phc_hash = service.hash_password(&password).await.unwrap();
/// assert!(service.verify_password(&password, &phc_hash).await.is_ok());
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct PasswordService {
    semaphore: Arc<Semaphore>,
    params: Params,
}

impl PasswordService {
    /// Creates a new `PasswordService` with custom concurrency limit and custom Argon2 parameters.
    ///
    /// # Arguments
    ///
    /// * `max_concurrent` - Maximum number of concurrent Argon2 computations permitted simultaneously.
    /// * `m_cost` - Memory size in KiB (e.g. `19456` or `65536`).
    /// * `t_cost` - Number of iterations (e.g. `2` or `3`).
    /// * `p_cost` - Parallelism degree (e.g. `1` or `4`).
    ///
    /// # Errors
    ///
    /// Returns [`PasswordError::HashFailed`] if the provided Argon2 parameters are invalid.
    pub fn new(
        max_concurrent: usize,
        m_cost: u32,
        t_cost: u32,
        p_cost: u32,
    ) -> Result<Self, PasswordError> {
        let params = Params::new(m_cost, t_cost, p_cost, None)
            .map_err(|e| PasswordError::HashFailed(e.to_string()))?;

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
            params,
        })
    }

    /// Hashes a plaintext password asynchronously respecting configured concurrency limits and Argon2 parameters.
    ///
    /// # Arguments
    ///
    /// * `pw` - Plaintext password wrapped in a [`SecretString`].
    ///
    /// # Errors
    ///
    /// Returns [`PasswordError::HashFailed`] if salt generation, Argon2 computation, or thread join fails.
    pub async fn hash_password(&self, pw: &SecretString) -> Result<String, PasswordError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| PasswordError::HashFailed(e.to_string()))?;

        let password = pw.clone();
        let params = self.params.clone();

        tokio::task::spawn_blocking(move || {
            let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
            argon2
                .hash_password(password.expose_secret().as_bytes())
                .map(|h| h.to_string())
                .map_err(|e| PasswordError::HashFailed(e.to_string()))
        })
        .await
        .map_err(|e| PasswordError::HashFailed(e.to_string()))?
    }

    /// Verifies a plaintext password against an Argon2 PHC formatted hash string asynchronously respecting concurrency limits.
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
    pub async fn verify_password(
        &self,
        pw: &SecretString,
        hash: &str,
    ) -> Result<(), PasswordError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| PasswordError::HashFailed(e.to_string()))?;

        let password = pw.clone();
        let hash = hash.to_string();

        tokio::task::spawn_blocking(move || {
            let parsed =
                PasswordHash::new(&hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;
            Argon2::default()
                .verify_password(password.expose_secret().as_bytes(), &parsed)
                .map_err(|_| PasswordError::VerificationFailed)
        })
        .await
        .map_err(|e| PasswordError::HashFailed(e.to_string()))?
    }
}

impl Default for PasswordService {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(
            cpus.max(2),
            Params::DEFAULT_M_COST,
            Params::DEFAULT_T_COST,
            Params::DEFAULT_P_COST,
        )
        .expect("Default Argon2 parameters must be valid")
    }
}
