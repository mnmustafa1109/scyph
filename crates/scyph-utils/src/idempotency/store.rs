//! Redis-backed idempotency store implementation.
//!
//! This module provides [`IdempotencyStore`] — a Redis-backed service that ensures any API
//! request submitted with a unique idempotency key is executed **at most once**, even when the
//! client retries the same request due to network timeouts or other transient errors.
//!
//! ## How It Works
//!
//! 1. **Atomic reservation** (`SET NX EX`): The first request atomically writes an `IN_PROGRESS`
//!    marker to Redis with an expiry. Concurrent duplicate requests see the marker and get a
//!    `409 Conflict` response.
//! 2. **Business logic execution**: The handler runs normally.
//! 3. **Cache result** (`SETEX`): On success, the response is serialized to JSON and stored in
//!    Redis under the same key with the configured (long) TTL.
//! 4. **Serve from cache** on retry: Subsequent requests with the same key get the cached response
//!    without re-executing business logic.
//! 5. **Cancel on failure**: If the handler returns an error, the key is deleted so the client
//!    can retry immediately.
//!
//! The `IN_PROGRESS` reservation uses a **short** expiry (`IdempotencyConfig::in_progress_ttl`,
//! default 60s) rather than the full result TTL. If the process crashes (or the task is dropped)
//! without reaching `complete`/`cancel`, the reservation self-heals within that window instead of
//! wedging the key as `409 Conflict` for the entire result TTL.
//!
//! Trade-off of that short window: if the handler itself runs **longer** than
//! `in_progress_ttl`, a client retry can pass `begin` and execute the closure a second time
//! concurrently. Completion is atomic (see [`IdempotencyStore::complete`]), so the *cached*
//! response is always the first completer's — but keep handlers shorter than the reservation
//! TTL to guarantee at-most-once execution.
//!
//! ```text
//! Client Request ──► begin(key)
//!                       ├── New ──────────────────► execute handler ──► complete_json / cancel
//!                       ├── InProgress ──────────► 409 Conflict
//!                       └── Seen(cached_json) ───► return cached response
//! ```
//!
//! ## Complete Axum Handler Example
//!
//! ```rust,ignore
//! use axum::{extract::{Extension, Json}, http::{HeaderMap, StatusCode}};
//! use scyph_utils::idempotency::{IdempotencyStore, IdempotencyError};
//! use scyph_core::AppError;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! pub struct ChargeRequest {
//!     pub amount_cents: u64,
//!     pub currency: String,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! pub struct ChargeResponse {
//!     pub transaction_id: String,
//!     pub status: String,
//! }
//!
//! pub async fn charge_handler(
//!     Extension(idem): Extension<IdempotencyStore>,
//!     headers: HeaderMap,
//!     Json(req): Json<ChargeRequest>,
//! ) -> Result<Json<ChargeResponse>, AppError> {
//!     // Extract idempotency key from request header
//!     let key = headers
//!         .get("Idempotency-Key")
//!         .and_then(|v| v.to_str().ok())
//!         .ok_or_else(|| AppError::BadRequest("Missing Idempotency-Key header".into()))?;
//!
//!     // execute() handles begin/complete/cancel automatically
//!     let response = idem
//!         .execute(key, || async {
//!             // Your business logic runs only once per unique key
//!             let txn_id = charge_payment(req.amount_cents, &req.currency).await?;
//!             Ok::<_, AppError>(ChargeResponse {
//!                 transaction_id: txn_id,
//!                 status: "succeeded".into(),
//!             })
//!         })
//!         .await?;
//!
//!     Ok(Json(response))
//! }
//! ```

use crate::idempotency::{config::IdempotencyConfig, error::IdempotencyError};
use redis::{AsyncCommands, Client, RedisResult};
use serde::Serialize;
use std::future::Future;

/// Outcome of an idempotency key lookup.
///
/// Returned by [`IdempotencyStore::begin`]. Drives the caller's decision on whether to
/// execute business logic, return a conflict response, or serve a cached result.
///
/// # Handling Each Variant
///
/// ```rust,ignore
/// match store.begin(&key).await? {
///     IdempotencyCheck::New => {
///         // Safe to execute — key was atomically reserved
///         let result = do_work().await?;
///         store.complete_json(&key, &result).await?;
///     }
///     IdempotencyCheck::InProgress => {
///         // Return 409 Conflict — another request is currently processing this key
///     }
///     IdempotencyCheck::Seen(cached_json) => {
///         // Return cached response — this exact request was already processed
///         let result: MyResponse = serde_json::from_str(&cached_json)?;
///     }
/// }
/// ```
///
/// In practice, prefer [`IdempotencyStore::execute`] which handles this match automatically.
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
    /// Lazily initialized, reused multiplexed connection. Multiplexed connections are
    /// cloneable and safe to use concurrently, so this amortizes one TCP connection.
    conn: std::sync::Arc<tokio::sync::Mutex<Option<redis::aio::MultiplexedConnection>>>,
}

impl IdempotencyStore {
    /// Internal marker payload written to Redis to denote an in-flight, incomplete request.
    pub const IN_PROGRESS_MARKER: &'static str = "IN_PROGRESS";

    /// Completes only when the key still holds the in-progress marker (or expired).
    ///
    /// Redis Lua script kept inline — a plain `SET` would let a late-retry execution
    /// overwrite the first completer's cached response.
    const COMPLETE_SCRIPT_SRC: &'static str = r#"
        local v = redis.call('GET', KEYS[1])
        if v == false or v == ARGV[1] then
            redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
            return 1
        end
        return 0
    "#;

    /// Deletes only while the key still holds the in-progress marker (or expired).
    ///
    /// A plain `DEL` would let a *failed* late-retry execution wipe the first completer's
    /// cached response, re-exposing the key to yet another execution.
    const CANCEL_SCRIPT_SRC: &'static str = r#"
        local v = redis.call('GET', KEYS[1])
        if v == false or v == ARGV[1] then
            redis.call('DEL', KEYS[1])
            return 1
        end
        return 0
    "#;

    /// Constructs a new `IdempotencyStore` with explicit [`IdempotencyConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if the Redis connection URL fails to parse.
    pub fn new(config: IdempotencyConfig) -> Result<Self, IdempotencyError> {
        let redis = Client::open(config.redis_url.as_str())?;
        Ok(Self {
            redis,
            config,
            conn: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Returns the cached multiplexed connection, opening it on first use.
    ///
    /// A single multiplexed connection supports concurrent commands; it is invalidated
    /// (recreated on demand) whenever a command fails.
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, IdempotencyError> {
        let mut guard = self.conn.lock().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        let conn = self.redis.get_multiplexed_async_connection().await?;
        *guard = Some(conn.clone());
        Ok(conn)
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
    /// Uses Redis atomic `SET key IN_PROGRESS NX EX ttl` to reserve the key with the short
    /// [`in_progress_ttl`](IdempotencyConfig::in_progress_ttl).
    /// Returns:
    /// - [`IdempotencyCheck::New`] if the key is new.
    /// - [`IdempotencyCheck::InProgress`] if a concurrent request is currently processing this key.
    /// - [`IdempotencyCheck::Seen`] if the key was previously completed with a cached response string.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn begin(&self, key: &str) -> Result<IdempotencyCheck, IdempotencyError> {
        let mut conn = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                *self.conn.lock().await = None;
                return Err(e);
            }
        };
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let res: RedisResult<Option<String>> = redis::cmd("SET")
            .arg(&redis_key)
            .arg(Self::IN_PROGRESS_MARKER)
            .arg("NX")
            .arg("EX")
            .arg(self.config.in_progress_ttl.as_secs())
            .query_async(&mut conn)
            .await;

        match res {
            Ok(Some(_)) => Ok(IdempotencyCheck::New),
            Ok(None) => {
                let cached: Option<String> = match conn.get(&redis_key).await {
                    Ok(cached) => cached,
                    Err(e) => {
                        *self.conn.lock().await = None;
                        return Err(IdempotencyError::Redis(e));
                    }
                };
                match cached.as_deref() {
                    Some(Self::IN_PROGRESS_MARKER) => Ok(IdempotencyCheck::InProgress),
                    Some(val) => Ok(IdempotencyCheck::Seen(val.to_string())),
                    None => Ok(IdempotencyCheck::New),
                }
            }
            Err(e) => {
                *self.conn.lock().await = None;
                Err(IdempotencyError::Redis(e))
            }
        }
    }

    /// Completes an idempotency check by caching the final response string.
    ///
    /// Atomically replaces the `IN_PROGRESS` marker with `response_body` through a Redis Lua
    /// script: the write only lands if the key is *still* the in-progress marker (or the
    /// reservation expired). If another execution already cached its result, this call returns
    /// [`IdempotencyError::Conflict`] and leaves the stored response untouched — the retry
    /// execution can never overwrite the first completer's answer.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Conflict`] if the key was already completed by another
    /// execution, or [`IdempotencyError::Redis`] if the Redis query fails.
    pub async fn complete(&self, key: &str, response_body: &str) -> Result<(), IdempotencyError> {
        let mut conn = self.get_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let script = redis::Script::new(Self::COMPLETE_SCRIPT_SRC);
        let res: RedisResult<i64> = script
            .key(&redis_key)
            .arg(Self::IN_PROGRESS_MARKER)
            .arg(response_body)
            .arg(self.config.ttl.as_secs())
            .invoke_async(&mut conn)
            .await;

        match res {
            Ok(0) => Err(IdempotencyError::Conflict(format!(
                "Key '{key}' was already completed by another request"
            ))),
            // Conflict is normal control flow, not a connection error — keep the connection.
            Ok(_) => Ok(()),
            Err(e) => {
                *self.conn.lock().await = None;
                Err(IdempotencyError::Redis(e))
            }
        }
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

    /// Cancels an in-progress idempotency key reservation (e.g. if handler business logic fails).
    ///
    /// Atomically removes the key **only while it still holds the `IN_PROGRESS` marker** (or is
    /// already gone) through a Redis Lua script. A completed response is never deleted — so a
    /// failed late-retry execution cannot wipe the first completer's cached result. Subsequent
    /// client retries can then attempt request execution immediately without waiting for TTL
    /// expiration.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::Redis`] if Redis query fails.
    pub async fn cancel(&self, key: &str) -> Result<(), IdempotencyError> {
        let mut conn = self.get_connection().await?;
        let redis_key = format!("{}:{}", self.config.prefix, key);

        let script = redis::Script::new(Self::CANCEL_SCRIPT_SRC);
        let res: RedisResult<i64> = script
            .key(&redis_key)
            .arg(Self::IN_PROGRESS_MARKER)
            .invoke_async(&mut conn)
            .await;

        match res {
            Ok(0) => {
                tracing::debug!(
                    key = key,
                    "Idempotency cancel skipped: response already completed and cached"
                );
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(e) => {
                *self.conn.lock().await = None;
                Err(IdempotencyError::Redis(e))
            }
        }
    }

    /// Executes an async operation with automated idempotency management.
    ///
    /// Replaces 25 lines of handler boilerplate with a 1-line execution wrapper:
    /// 1. Reserves `key` in Redis (`begin`).
    /// 2. If `key` is new, executes closure `f()`.
    /// 3. If `f()` succeeds (`Ok(val)`), serializes and caches `val` in Redis (`complete_json`).
    ///    Completion is atomic — if a later overlap (see the module-level duplicate-window note)
    ///    already cached its result, the computed value is still returned to the caller, just not
    ///    stored twice.
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
                        match &e {
                            IdempotencyError::Conflict(_) => {
                                // A late retry that passed `begin` completed first; its cached
                                // response stays, ours is discarded. Not an error.
                                tracing::warn!(
                                    key = key,
                                    "Idempotency key completed by a concurrent retry; keeping its cached response"
                                );
                            }
                            _ => {
                                tracing::error!(key = key, error = %e, "Failed to cache idempotency response");
                            }
                        }
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
