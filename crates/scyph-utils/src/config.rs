//! Configuration structures and environment variable loaders for `scyph-utils`.

use std::{env, time::Duration};

/// Configuration options for Redis-backed API idempotency store.
#[derive(Debug, Clone)]
pub struct IdempotencyConfig {
    /// Redis connection URL string (e.g. `"redis://127.0.0.1:6379"`).
    pub redis_url: String,
    /// Time-to-live duration for stored idempotency keys and cached response payloads.
    pub ttl: Duration,
    /// Redis key namespace prefix (default: `"idem"`).
    pub prefix: String,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            ttl: Duration::from_secs(86400), // 24 hours
            prefix: "idem".to_string(),
        }
    }
}

impl IdempotencyConfig {
    /// Loads configuration options from environment variables:
    ///
    /// - `IDEMPOTENCY_REDIS_URL` / `REDIS_URL` *(Optional)*: Redis connection string (defaults to `"redis://127.0.0.1:6379"`).
    /// - `IDEMPOTENCY_TTL_SECS` *(Optional)*: Key expiration in seconds (defaults to `86400`).
    /// - `IDEMPOTENCY_PREFIX` *(Optional)*: Redis namespace key prefix (defaults to `"idem"`).
    pub fn from_env() -> Self {
        let redis_url = env::var("IDEMPOTENCY_REDIS_URL")
            .or_else(|_| env::var("REDIS_URL"))
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let ttl_secs = env::var("IDEMPOTENCY_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86400);

        let prefix = env::var("IDEMPOTENCY_PREFIX").unwrap_or_else(|_| "idem".to_string());

        Self {
            redis_url,
            ttl: Duration::from_secs(ttl_secs),
            prefix,
        }
    }

    /// Builder method to override the Redis connection URL.
    pub fn with_redis_url(mut self, url: impl Into<String>) -> Self {
        self.redis_url = url.into();
        self
    }

    /// Builder method to override key time-to-live duration.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
}
