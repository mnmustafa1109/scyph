//! Environment configuration for API idempotency checks.

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

impl IdempotencyConfig {
    /// Default time-to-live duration for stored idempotency keys (24 hours).
    pub const DEFAULT_TTL: Duration = Duration::from_secs(86400);
    /// Default Redis key namespace prefix.
    pub const DEFAULT_PREFIX: &'static str = "idem";
    /// Default fallback Redis connection URL.
    pub const DEFAULT_REDIS_URL: &'static str = "redis://127.0.0.1:6379";

    /// Loads configuration options from environment variables:
    ///
    /// - `IDEMPOTENCY_REDIS_URL` / `REDIS_URL` *(Optional)*: Redis connection string (defaults to `DEFAULT_REDIS_URL`).
    /// - `IDEMPOTENCY_TTL_SECS` *(Optional)*: Key expiration in seconds (defaults to `DEFAULT_TTL`).
    /// - `IDEMPOTENCY_PREFIX` *(Optional)*: Redis namespace key prefix (defaults to `DEFAULT_PREFIX`).
    pub fn from_env() -> Self {
        let redis_url = env::var("IDEMPOTENCY_REDIS_URL")
            .or_else(|_| env::var("REDIS_URL"))
            .unwrap_or_else(|_| Self::DEFAULT_REDIS_URL.to_string());

        let ttl_secs = env::var("IDEMPOTENCY_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Self::DEFAULT_TTL);

        let prefix =
            env::var("IDEMPOTENCY_PREFIX").unwrap_or_else(|_| Self::DEFAULT_PREFIX.to_string());

        Self {
            redis_url,
            ttl: ttl_secs,
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

    /// Builder method to override the key namespace prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            redis_url: Self::DEFAULT_REDIS_URL.to_string(),
            ttl: Self::DEFAULT_TTL,
            prefix: Self::DEFAULT_PREFIX.to_string(),
        }
    }
}
