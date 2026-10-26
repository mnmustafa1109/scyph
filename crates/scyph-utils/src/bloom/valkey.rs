//! Valkey/Redis-backed Bloom filter (bitmap + Lua, no module required).

use super::{MAX_BITS, bloom_params, hash_indices, hash_pair};
use redis::{AsyncCommands, Client, Script};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Items per atomic Lua batch when bulk-adding.
const BATCH_CHUNK: usize = 512;

const ADD_SCRIPT: &str = r#"
local all = 1
for i = 1, #ARGV do
    local off = tonumber(ARGV[i])
    if redis.call('GETBIT', KEYS[1], off) == 0 then
        all = 0
    end
    redis.call('SETBIT', KEYS[1], off, 1)
end
return all
"#;

const EXISTS_SCRIPT: &str = r#"
for i = 1, #ARGV do
    if redis.call('GETBIT', KEYS[1], tonumber(ARGV[i])) == 0 then
        return 0
    end
end
return 1
"#;

/// Errors returned by [`ValkeyBloomFilter`].
#[derive(Debug, thiserror::Error)]
pub enum ValkeyBloomError {
    /// A Redis/Valkey command failed.
    #[error("redis/valkey error: {0}")]
    Redis(#[from] redis::RedisError),
    /// The connection URL could not be parsed.
    #[error("invalid connection url: {0}")]
    Url(String),
    /// Required configuration is missing.
    #[error("configuration error: {0}")]
    Config(String),
}

/// A Bloom filter whose bits live in a Valkey/Redis string key.
///
/// The actual Redis key is derived from the `key` argument plus the format
/// version, computed bit size, and hash count (`{key}:bloom:v1:{bits}:{hashes}`).
/// Different sizing uses a different key. When changing the hash function or bit
/// layout, bump the `v1` segment so old bits are not read with new positions.
/// The key has no TTL; [`ValkeyBloomFilter::clear`] deletes only the current
/// key, so delete old sized keys yourself when you change parameters.
///
/// # Example
///
/// ```rust,no_run
/// use scyph_utils::bloom::ValkeyBloomFilter;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let filter = ValkeyBloomFilter::from_env("crm:emails", 1_000_000, 0.01)?;
/// filter.ping().await?;
/// filter.add(&"user@example.com").await?;
/// assert!(filter.contains(&"user@example.com").await?);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ValkeyBloomFilter {
    redis: Client,
    key: String,
    bit_len: usize,
    hash_count: u32,
    conn: Arc<Mutex<Option<redis::aio::MultiplexedConnection>>>,
}

impl ValkeyBloomFilter {
    /// Creates a filter sized for `expected_items` at the given false-positive rate.
    pub fn new(
        redis_url: &str,
        key: impl Into<String>,
        expected_items: usize,
        false_positive_rate: f64,
    ) -> Result<Self, ValkeyBloomError> {
        let redis = Client::open(redis_url).map_err(|e| ValkeyBloomError::Url(e.to_string()))?;
        let (m, k) = bloom_params(expected_items, false_positive_rate);
        let m = m.clamp(64, MAX_BITS);
        Ok(Self {
            redis,
            // Bump `v1` whenever the hash function or bit layout changes.
            key: format!("{}:bloom:v1:{m}:{k}", key.into()),
            bit_len: m,
            hash_count: k,
            conn: Arc::new(Mutex::new(None)),
        })
    }

    /// Creates a filter sized to the iterator and warms it.
    ///
    /// Collects the iterator once to learn its length, so peak memory is the
    /// full item set. Because the Redis key includes the derived size, a
    /// changing item count creates a new key on each run. For a durable filter
    /// whose key must stay stable, use [`Self::from_items_with_size`] with an
    /// explicit `expected_items`.
    pub async fn from_items<U, I>(
        redis_url: &str,
        key: impl Into<String>,
        false_positive_rate: f64,
        items: I,
    ) -> Result<Self, ValkeyBloomError>
    where
        U: std::hash::Hash,
        I: IntoIterator<Item = U>,
    {
        let items: Vec<U> = items.into_iter().collect();
        Self::from_items_with_size(
            redis_url,
            key,
            items.len().max(1),
            false_positive_rate,
            items,
        )
        .await
    }

    /// Creates a filter with a fixed `expected_items` and warms it.
    ///
    /// Use this for durable filters so a growing item count does not change the
    /// derived Redis key. The iterator is streamed to Valkey in batches.
    ///
    /// ```rust,no_run
    /// use scyph_utils::bloom::ValkeyBloomFilter;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let filter = ValkeyBloomFilter::from_items_with_size(
    ///     "redis://127.0.0.1:6379",
    ///     "crm:emails",
    ///     1_000_000,
    ///     0.01,
    ///     vec!["user@example.com".to_string()],
    /// )
    /// .await?;
    /// assert!(filter.contains(&"user@example.com").await?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_items_with_size<U, I>(
        redis_url: &str,
        key: impl Into<String>,
        expected_items: usize,
        false_positive_rate: f64,
        items: I,
    ) -> Result<Self, ValkeyBloomError>
    where
        U: std::hash::Hash,
        I: IntoIterator<Item = U>,
    {
        let filter = Self::new(redis_url, key, expected_items, false_positive_rate)?;
        filter.add_many(items).await?;
        Ok(filter)
    }

    /// Returns the derived Redis key this filter reads and writes.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Checks connectivity with a `PING`.
    ///
    /// Call this once at startup to fail fast on a bad URL instead of waiting
    /// for the first `add`/`contains`.
    pub async fn ping(&self) -> Result<(), ValkeyBloomError> {
        let mut conn = self.connection().await?;
        if let Err(e) = redis::cmd("PING").query_async::<String>(&mut conn).await {
            *self.conn.lock().await = None;
            return Err(e.into());
        }
        Ok(())
    }

    /// Creates a filter from `VALKEY_URL`/`REDIS_URL`.
    ///
    /// # Errors
    ///
    /// Returns [`ValkeyBloomError::Config`] if neither variable is set.
    pub fn from_env(
        key: impl Into<String>,
        expected_items: usize,
        false_positive_rate: f64,
    ) -> Result<Self, ValkeyBloomError> {
        let url = std::env::var("VALKEY_URL")
            .or_else(|_| std::env::var("REDIS_URL"))
            .map_err(|_| ValkeyBloomError::Config("set VALKEY_URL or REDIS_URL".to_string()))?;
        Self::new(&url, key, expected_items, false_positive_rate)
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, ValkeyBloomError> {
        let mut guard = self.conn.lock().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        let conn = self.redis.get_multiplexed_async_connection().await?;
        *guard = Some(conn.clone());
        Ok(conn)
    }

    fn offsets<T: std::hash::Hash>(&self, item: &T) -> Vec<usize> {
        let (h1, h2) = hash_pair(item);
        hash_indices(h1, h2, self.bit_len, self.hash_count).collect()
    }

    /// Sets the bits for many items, batching one atomic Lua call per chunk.
    pub async fn add_many<U, I>(&self, items: I) -> Result<(), ValkeyBloomError>
    where
        U: std::hash::Hash,
        I: IntoIterator<Item = U>,
    {
        let mut batch = Vec::with_capacity(BATCH_CHUNK);
        for item in items {
            batch.push(item);
            if batch.len() == BATCH_CHUNK {
                self.flush_batch(&batch).await?;
                batch.clear();
            }
        }
        if !batch.is_empty() {
            self.flush_batch(&batch).await?;
        }
        Ok(())
    }

    async fn flush_batch<U: std::hash::Hash>(&self, items: &[U]) -> Result<(), ValkeyBloomError> {
        // One Lua call per chunk, so the whole chunk lands atomically and a
        // concurrent `contains` never observes half an item.
        let script = Script::new(ADD_SCRIPT);
        let mut cmd = script.key(&self.key);
        for item in items {
            for off in self.offsets(item) {
                cmd.arg(off);
            }
        }
        let mut conn = self.connection().await?;
        let _: i64 = match cmd.invoke_async(&mut conn).await {
            Ok(v) => v,
            Err(e) => {
                *self.conn.lock().await = None;
                return Err(e.into());
            }
        };
        Ok(())
    }

    /// Sets the bits for `item`. Returns `Ok(true)` if it was possibly already present.
    pub async fn add<T: std::hash::Hash>(&self, item: &T) -> Result<bool, ValkeyBloomError> {
        let offsets = self.offsets(item);
        let script = Script::new(ADD_SCRIPT);
        let mut cmd = script.key(&self.key);
        for off in &offsets {
            cmd.arg(*off);
        }
        let mut conn = self.connection().await?;
        let already: i64 = match cmd.invoke_async(&mut conn).await {
            Ok(v) => v,
            Err(e) => {
                *self.conn.lock().await = None;
                return Err(e.into());
            }
        };
        Ok(already == 1)
    }

    /// Returns `Ok(true)` if `item` might be in the set.
    pub async fn contains<T: std::hash::Hash>(&self, item: &T) -> Result<bool, ValkeyBloomError> {
        let offsets = self.offsets(item);
        let script = Script::new(EXISTS_SCRIPT);
        let mut cmd = script.key(&self.key);
        for off in &offsets {
            cmd.arg(*off);
        }
        let mut conn = self.connection().await?;
        let present: i64 = match cmd.invoke_async(&mut conn).await {
            Ok(v) => v,
            Err(e) => {
                *self.conn.lock().await = None;
                return Err(e.into());
            }
        };
        Ok(present == 1)
    }

    /// Deletes the underlying key, resetting the filter.
    pub async fn clear(&self) -> Result<(), ValkeyBloomError> {
        let mut conn = self.connection().await?;
        if let Err(e) = conn.del::<_, ()>(&self.key).await {
            *self.conn.lock().await = None;
            return Err(e.into());
        }
        Ok(())
    }
}
