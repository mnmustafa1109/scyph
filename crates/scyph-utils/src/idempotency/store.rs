//! Redis-backed idempotency store implementation.

use crate::idempotency::{config::IdempotencyConfig, error::IdempotencyError};
use redis::{AsyncCommands, Client, RedisResult};
use serde::Serialize;
use std::future::Future;

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
                let cached: Option<String> = conn.get(&redis_key).await?;
                match cached.as_deref() {
                    Some(Self::IN_PROGRESS_MARKER) => Ok(IdempotencyCheck::InProgress),
                    Some(val) => Ok(IdempotencyCheck::Seen(val.to_string())),
                    None => Ok(IdempotencyCheck::New),
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

    /// Executes an async operation with automated idempotency management.
    ///
    /// Replaces 25 lines of handler boilerplate with a 1-line execution wrapper:
    /// 1. Reserves `key` in Redis (`begin`).
    /// 2. If `key` is new, executes closure `f()`.
    /// 3. If `f()` succeeds (`Ok(val)`), serializes and caches `val` in Redis (`complete_json`).
    /// 4. If `f()` fails (`Err(err)`), automatically cancels the key reservation (`cancel`).
    /// 5. If `key` was previously seen, returns the cached response deserialized from JSON.
    /// 6. If a concurrent request is in-flight, returns [`IdempotencyError::Conflict`].
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Conflict`] if a request with `key` is in-flight,
    /// [`IdempotencyError::Json`] / [`IdempotencyError::Redis`] on storage failures,
    /// or passes through the inner error `E`.
    pub async fn execute<T, E, F, Fut>(&self, key: &str, f: F) -> Result<T, E>
    where
        T: Serialize + serde::de::DeserializeOwned,
        E: From<IdempotencyError>,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        match self.begin(key).await {
            Ok(IdempotencyCheck::New) => match f().await {
                Ok(val) => {
                    if let Err(e) = self.complete_json(key, &val).await {
                        tracing::error!(key = key, error = %e, "Failed to cache idempotency response");
                    }
                    Ok(val)
                }
                Err(err) => {
                    let _ = self.cancel(key).await;
                    Err(err)
                }
            },
            Ok(IdempotencyCheck::InProgress) => Err(E::from(IdempotencyError::Conflict(format!(
                "A request with key '{key}' is currently in progress"
            )))),
            Ok(IdempotencyCheck::Seen(cached_json)) => {
                let val = serde_json::from_str::<T>(&cached_json)
                    .map_err(|e| E::from(IdempotencyError::Json(e)))?;
                Ok(val)
            }
            Err(e) => Err(E::from(e)),
        }
    }
}
