//! Realtime broadcaster configuration parameters.

use std::env;

use crate::error::RealtimeError;

/// Configuration options for [`RealtimeBroadcaster`](crate::RealtimeBroadcaster).
#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    /// Redis connection URL string (e.g., `"redis://127.0.0.1:6379"`).
    pub redis_url: String,
    /// Pub/sub Redis channel prefix (e.g., `"scyph:realtime"`).
    pub channel_prefix: String,
    /// Interval in seconds between reconnect attempts when subscriber stream disconnects.
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
    /// Constructs a [`RealtimeConfig`] by reading environment variables:
    /// - `REDIS_URL` (or `REALTIME_REDIS_URL`, defaulting to `"redis://127.0.0.1:6379"`)
    /// - `REALTIME_CHANNEL_PREFIX` (defaulting to `"scyph:realtime"`)
    /// - `REALTIME_RECONNECT_INTERVAL_SECS` (defaulting to `1`)
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError::Configuration`] if environment parsing fails.
    pub fn from_env() -> Result<Self, RealtimeError> {
        let redis_url = env::var("REALTIME_REDIS_URL")
            .or_else(|_| env::var("REDIS_URL"))
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let channel_prefix = env::var("REALTIME_CHANNEL_PREFIX")
            .unwrap_or_else(|_| "scyph:realtime".to_string());

        let reconnect_interval_secs = env::var("REALTIME_RECONNECT_INTERVAL_SECS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .map_err(|e| RealtimeError::Configuration(format!("REALTIME_RECONNECT_INTERVAL_SECS must be a valid u64: {e}")))?;

        Ok(Self {
            redis_url,
            channel_prefix,
            reconnect_interval_secs,
        })
    }

    /// Constructs a new [`RealtimeConfig`] with custom Redis URL and channel prefix.
    pub fn new(redis_url: impl Into<String>, channel_prefix: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
            channel_prefix: channel_prefix.into(),
            ..Default::default()
        }
    }
}
