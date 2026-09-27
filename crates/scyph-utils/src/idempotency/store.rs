//! Redis-backed idempotency store implementation.

use crate::idempotency::{config::IdempotencyConfig, error::IdempotencyError};
use redis::{AsyncCommands, Client, RedisResult};
use serde::Serialize;

/// Outcome of an idempotency key lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyCheck {
    /// The request key has not been seen before. Proceed with execution.
    New,
    /// A concurrent request with the same idempotency key is currently executing.
    InProgress,
    /// The request key was previously executed and its cached response string is returned.
    Seen(String),
}

/// Redis-backed idempotency service enforcing single execution of API requests.
///
/// Features **atomic key reservation** (`SET NX EX`) and **in-flight concurrency protection** (`IN_PROGRESS`).
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
///         IdempotencyCheck::InProgress => {
///             println!("Request currently processing by peer thread...");
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
    /// Internal marker payload written to Redis to denote an in-flight, incomplete request.
    pub const IN_PROGRESS_MARKER: &'static str = "IN_PROGRESS";

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
    /// Uses Redis atomic `SET key IN_PROGRESS NX EX ttl` to reserve the key.
    /// Returns:
    /// - [`IdempotencyCheck::New`] if the key is new.
    /// - [`IdempotencyCheck::InProgress`] if a concurrent request is currently processing this key.
    /// - [`IdempotencyCheck::Seen`] if the key was previously completed with a cached response string.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn begin(&self, key: &str) -> Result<IdempotencyCheck, IdempotencyError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let res: RedisResult<Option<String>> = redis::cmd("SET")
            .arg(&redis_key)
            .arg(Self::IN_PROGRESS_MARKER)
            .arg("NX")
            .arg("EX")
            .arg(self.config.ttl.as_secs())
            .query_async(&mut conn)
            .await;

        match res {
            Ok(Some(_)) => Ok(IdempotencyCheck::New),
            Ok(None) => {
                let cached: String = conn.get(&redis_key).await?;
                if cached == Self::IN_PROGRESS_MARKER {
                    Ok(IdempotencyCheck::InProgress)
                } else {
                    Ok(IdempotencyCheck::Seen(cached))
                }
            }
            Err(e) => Err(IdempotencyError::Redis(e)),
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

    /// Cancels or removes an in-progress idempotency key reservation (e.g. if handler business logic fails).
    ///
    /// Allows subsequent client retries to attempt request execution immediately without waiting for TTL expiration.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn cancel(&self, key: &str) -> Result<(), IdempotencyError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let _: () = conn.del(&redis_key).await?;
        Ok(())
    }
}
