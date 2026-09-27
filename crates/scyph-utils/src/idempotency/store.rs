//! Redis-backed idempotency store implementation.

use crate::idempotency::{config::IdempotencyConfig, error::IdempotencyError};
use redis::{AsyncCommands, Client};
use serde::Serialize;

/// Outcome of an idempotency key lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyCheck {
    /// The request key has not been seen before. Proceed with execution.
    New,
    /// The request key was previously executed and its cached response string is returned.
    Seen(String),
}

/// Redis-backed idempotency service enforcing single execution of API requests.
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_utils::idempotency::{IdempotencyStore, IdempotencyCheck, IdempotencyConfig};
///
/// async fn handle_payment(key: &str) -> Result<(), Box<dyn std::error::Error>> {
///     let config = IdempotencyConfig::from_env();
///     let store = IdempotencyStore::new(config)?;
///
///     match store.begin(key).await? {
///         IdempotencyCheck::New => {
///             let response = r#"{"status":"success","transaction_id":"tx_123"}"#;
///             store.complete(key, response).await?;
///         }
///         IdempotencyCheck::Seen(cached_response) => {
///             println!("Returning cached response: {cached_response}");
///         }
///     }
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug)]
pub struct IdempotencyStore {
    redis: Client,
    config: IdempotencyConfig,
}

impl IdempotencyStore {
    /// Constructs a new `IdempotencyStore` with explicit [`IdempotencyConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if the Redis connection URL fails to parse.
    pub fn new(config: IdempotencyConfig) -> Result<Self, IdempotencyError> {
        let redis = Client::open(config.redis_url.as_str())?;
        Ok(Self { redis, config })
    }

    /// Constructs an `IdempotencyStore` from environment variables using [`IdempotencyConfig::from_env`].
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if opening the Redis client fails.
    pub fn from_env() -> Result<Self, IdempotencyError> {
        let config = IdempotencyConfig::from_env();
        Self::new(config)
    }

    /// Begins an idempotency check for the given unique key.
    ///
    /// Uses Redis `SETNX` (Set if Not Exists) to atomically reserve the key.
    /// Returns [`IdempotencyCheck::New`] if the key is new, or [`IdempotencyCheck::Seen`]
    /// if the key was previously completed.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn begin(&self, key: &str) -> Result<IdempotencyCheck, IdempotencyError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let is_new: bool = conn.set_nx(&redis_key, "").await?;
        if is_new {
            let _: () = conn
                .expire(&redis_key, self.config.ttl.as_secs() as i64)
                .await?;
            Ok(IdempotencyCheck::New)
        } else {
            let cached: String = conn.get(&redis_key).await?;
            Ok(IdempotencyCheck::Seen(cached))
        }
    }

    /// Completes an idempotency check by caching the final response string.
    ///
    /// Sets key value to `response_body` with expiration matching configured TTL (`SETEX`).
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn complete(&self, key: &str, response_body: &str) -> Result<(), IdempotencyError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let _: () = conn
            .set_ex(&redis_key, response_body, self.config.ttl.as_secs())
            .await?;
        Ok(())
    }

    /// Completes an idempotency check by serializing a Rust data structure to JSON.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Json`] if JSON serialization fails, or [`IdempotencyError::Redis`] if Redis store fails.
    pub async fn complete_json<T: Serialize>(
        &self,
        key: &str,
        data: &T,
    ) -> Result<(), IdempotencyError> {
        let json = serde_json::to_string(data)?;
        self.complete(key, &json).await
    }
}
