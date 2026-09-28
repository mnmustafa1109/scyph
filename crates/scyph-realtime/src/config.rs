//! Realtime broadcaster configuration parameters.

use std::env;

use crate::error::RealtimeError;

/// Configuration options for [`RealtimeBroadcaster`](crate::RealtimeBroadcaster).
///
/// Controls Redis connectivity, Pub/Sub channel naming, and reconnection behavior.
/// Can be constructed from environment variables ([`from_env`](Self::from_env)),
/// with explicit values ([`new`](Self::new)), or with the built-in [`Default`] implementation.
///
/// # Environment Variables
///
/// | Variable | Description | Default |
/// |---|---|---|
/// | `REALTIME_REDIS_URL` | Redis connection URL | `redis://127.0.0.1:6379` |
/// | `REDIS_URL` | Fallback Redis URL (if `REALTIME_REDIS_URL` is not set) | `redis://127.0.0.1:6379` |
/// | `REALTIME_CHANNEL_PREFIX` | Pub/Sub channel prefix string | `scyph:realtime` |
/// | `REALTIME_RECONNECT_INTERVAL_SECS` | Seconds to wait between reconnect attempts | `1` |
///
/// The full Redis channel name used for broadcasting is `{channel_prefix}:events`.
///
/// # Examples
///
/// ## From environment variables
///
/// ```rust,ignore
/// use scyph_realtime::RealtimeConfig;
///
/// // Reads REALTIME_REDIS_URL / REDIS_URL, REALTIME_CHANNEL_PREFIX, REALTIME_RECONNECT_INTERVAL_SECS
/// let config = RealtimeConfig::from_env()?;
/// ```
///
/// ## Explicit construction
///
/// ```rust
/// use scyph_realtime::RealtimeConfig;
///
/// let config = RealtimeConfig::new("redis://127.0.0.1:6379", "myapp:realtime");
/// assert_eq!(config.channel_prefix, "myapp:realtime");
/// ```
///
/// ## Full broadcaster setup
///
/// ```rust,ignore
/// use scyph_realtime::{RealtimeConfig, RealtimeBroadcaster};
/// use tokio_util::sync::CancellationToken;
///
/// let config = RealtimeConfig {
///     redis_url: "redis://redis-host:6379".to_string(),
///     channel_prefix: "prod:realtime".to_string(),
///     reconnect_interval_secs: 5,
/// };
///
/// let broadcaster = RealtimeBroadcaster::new(config)?;
/// let cancel = CancellationToken::new();
/// let _handle = broadcaster.start_subscriber(cancel.clone());
/// ```
#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    /// Redis connection URL string.
    ///
    /// Supports standard Redis URL formats including authentication and database selection:
    /// - `redis://127.0.0.1:6379` — basic local connection
    /// - `redis://:password@host:6379` — with password
    /// - `redis://host:6379/1` — with database index
    /// - `rediss://host:6379` — TLS-encrypted connection
    pub redis_url: String,

    /// Pub/Sub Redis channel prefix.
    ///
    /// All events are published to and consumed from the channel `{channel_prefix}:events`.
    /// Use distinct prefixes to isolate environments (e.g., `"dev:realtime"` vs `"prod:realtime"`).
    pub channel_prefix: String,

    /// Interval in seconds between reconnect attempts when the subscriber stream disconnects.
    ///
    /// When the Redis subscriber loop encounters a connection error, it waits this many seconds
    /// before attempting to reconnect. Increasing this value reduces reconnect storm pressure
    /// during Redis outages at the cost of delayed message delivery resumption.
    pub reconnect_interval_secs: u64,
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            channel_prefix: "scyph:realtime".to_string(),
            reconnect_interval_secs: 1,
        }
    }
}

impl RealtimeConfig {
    /// Constructs a [`RealtimeConfig`] by reading environment variables.
    ///
    /// Variable resolution order:
    /// 1. `REALTIME_REDIS_URL` — primary Redis URL
    /// 2. `REDIS_URL` — shared Redis URL fallback (useful when the same Redis instance is shared across crates)
    /// 3. Hardcoded default: `redis://127.0.0.1:6379`
    ///
    /// | Variable | Description | Default |
    /// |---|---|---|
    /// | `REALTIME_REDIS_URL` or `REDIS_URL` | Redis server URL | `redis://127.0.0.1:6379` |
    /// | `REALTIME_CHANNEL_PREFIX` | Pub/Sub channel prefix | `scyph:realtime` |
    /// | `REALTIME_RECONNECT_INTERVAL_SECS` | Reconnect wait time in seconds | `1` |
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError::Configuration`] if `REALTIME_RECONNECT_INTERVAL_SECS` contains
    /// a value that cannot be parsed as a `u64`.
    pub fn from_env() -> Result<Self, RealtimeError> {
        let redis_url = env::var("REALTIME_REDIS_URL")
            .or_else(|_| env::var("REDIS_URL"))
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let channel_prefix =
            env::var("REALTIME_CHANNEL_PREFIX").unwrap_or_else(|_| "scyph:realtime".to_string());

        let reconnect_interval_secs = env::var("REALTIME_RECONNECT_INTERVAL_SECS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .map_err(|e| {
                RealtimeError::Configuration(format!(
                    "REALTIME_RECONNECT_INTERVAL_SECS must be a valid u64: {e}"
                ))
            })?;

        Ok(Self {
            redis_url,
            channel_prefix,
            reconnect_interval_secs,
        })
    }

    /// Constructs a new [`RealtimeConfig`] with custom Redis URL and channel prefix.
    ///
    /// Uses the default `reconnect_interval_secs` of `1`. To customize the reconnect interval,
    /// construct [`RealtimeConfig`] directly using struct syntax.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_realtime::RealtimeConfig;
    ///
    /// let config = RealtimeConfig::new("redis://localhost:6379", "myapp:realtime");
    /// assert_eq!(config.reconnect_interval_secs, 1);
    /// ```
    pub fn new(redis_url: impl Into<String>, channel_prefix: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
            channel_prefix: channel_prefix.into(),
            ..Default::default()
        }
    }
}
