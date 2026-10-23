//! Password service for concurrency-managed password hashing and verification.

use argon2::{
    Argon2, Params,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::password::PasswordError;

/// Production-grade password hashing service with configurable concurrency and Argon2 parameters.
///
/// Encapsulates a shared [`Semaphore`] concurrency limiter, Argon2 cost parameters, and an
/// optional **Argon2 pepper** (`ARGON2_SECRET`): a server-side secret mixed into every hash
/// computation but never stored in the database, which neutralizes hashes exposed by a
/// database leak (verification fails without the same pepper configured).
///
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
    /// Optional pepper applied inside the Argon2 computation ("secret key" per the PHC spec).
    secret: Option<Arc<SecretString>>,
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
            secret: None,
        })
    }

    /// Creates a `PasswordService` with the default parameters and reads the optional
    /// Argon2 pepper from the `ARGON2_SECRET` environment variable.
    ///
    /// The pepper must be identical on every replica of the service for verification of a
    /// single stored hash to succeed anywhere. Rotating it instantly invalidates every
    /// stored password hash — hashes and verifications both require the same pepper.
    pub fn from_env() -> Self {
        // Pepper is read once at startup; runtime env mutation is intentionally not supported.
        match std::env::var("ARGON2_SECRET") {
            Ok(secret) => Self::default().with_secret(SecretString::from(secret)),
            Err(_) => Self::default(),
        }
    }

    /// Sets the Argon2 pepper explicitly, as an alternative to the
    /// `ARGON2_SECRET` environment variable.
    pub fn with_secret(mut self, secret: SecretString) -> Self {
        self.secret = Some(Arc::new(secret));
        self
    }
    /// Builds an Argon2id instance, applying the configured pepper (PHC "secret key") when present.
    fn build_argon2<'k>(
        params: &Params,
        secret: Option<&'k SecretString>,
    ) -> Result<Argon2<'k>, PasswordError> {
        match secret {
            Some(secret) => Argon2::new_with_secret(
                secret.expose_secret().as_bytes(),
                argon2::Algorithm::Argon2id,
                argon2::Version::V0x13,
                params.clone(),
            )
            .map_err(|e| PasswordError::HashFailed(e.to_string())),
            None => Ok(Argon2::new(
                argon2::Algorithm::Argon2id,
                argon2::Version::V0x13,
                params.clone(),
            )),
        }
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
        let pepper = self.secret.clone();

        tokio::task::spawn_blocking(move || {
            let argon2 = Self::build_argon2(&params, pepper.as_deref())?;
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
        let params = self.params.clone();
        let pepper = self.secret.clone();

        tokio::task::spawn_blocking(move || {
            let parsed =
                PasswordHash::new(&hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;
            // The active pepper (if any) must match the one that produced the stored hash;
            // cost parameters come from the PHC string, so `self.params` is not used here.
            let argon2 = Self::build_argon2(&params, pepper.as_deref())?;
            argon2
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
