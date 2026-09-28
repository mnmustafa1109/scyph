//! PostgreSQL database connection pool builder and configuration.

use crate::DbError;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, time::Duration};
use tracing::info;

/// Configuration options for PostgreSQL connection pool.
#[derive(Clone, Debug)]
pub struct DbConfig {
    /// PostgreSQL connection URL string wrapped in [`SecretString`].
    pub database_url: SecretString,
    /// Maximum concurrent connections in the pool (default: 5).
    pub max_connections: u32,
    /// Minimum idle connections maintained in the pool (default: 1).
    pub min_connections: u32,
    /// Connection acquisition timeout (default: 30 seconds).
    pub acquire_timeout: Duration,
    /// Idle connection timeout before reaping (default: 10 minutes).
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of an individual connection (default: 30 minutes).
    pub max_lifetime: Option<Duration>,
}

impl DbConfig {
    /// Constructs a [`DbConfig`] with the provided database connection URL and sensible production defaults.
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: SecretString::from(database_url.into()),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        }
    }

    /// Constructs a [`DbConfig`] by reading standard environment variables:
    /// - `DATABASE_URL` (Required)
    /// - `MAX_CONNECTIONS` (Optional, defaults to 5)
    /// - `MIN_CONNECTIONS` (Optional, defaults to 1)
    /// - `DB_ACQUIRE_TIMEOUT_SECS` (Optional, defaults to 30)
    pub fn from_env() -> Result<Self, DbError> {
        let url = SecretString::from(env::var("DATABASE_URL").map_err(|_| {
            DbError::Configuration("DATABASE_URL environment variable must be set".into())
        })?);

        let max_conn = env::var("MAX_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .map_err(|e| {
                DbError::Configuration(format!("MAX_CONNECTIONS must be a valid u32: {e}"))
            })?;

        let min_conn = env::var("MIN_CONNECTIONS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u32>()
            .map_err(|e| {
                DbError::Configuration(format!("MIN_CONNECTIONS must be a valid u32: {e}"))
            })?;

        let acquire_timeout_secs = env::var("DB_ACQUIRE_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|e| {
                DbError::Configuration(format!("DB_ACQUIRE_TIMEOUT_SECS must be a valid u64: {e}"))
            })?;

        Ok(Self {
            database_url: url,
            max_connections: max_conn,
            min_connections: min_conn,
            acquire_timeout: Duration::from_secs(acquire_timeout_secs),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        })
    }

    /// Sets the maximum concurrent connections.
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Sets the minimum idle connections.
    pub fn with_min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Sets the connection acquisition timeout.
    pub fn with_acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// Sets the idle connection reap timeout.
    pub fn with_idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Sets the maximum lifetime of an individual connection.
    pub fn with_max_lifetime(mut self, lifetime: Option<Duration>) -> Self {
        self.max_lifetime = lifetime;
        self
    }
}

/// Constructs a PostgreSQL connection pool (`PgPool`) from a [`DbConfig`].
pub async fn build_pool_with_config(config: &DbConfig) -> Result<PgPool, DbError> {
    info!(
        max_connections = config.max_connections,
        min_connections = config.min_connections,
        acquire_timeout_secs = config.acquire_timeout.as_secs(),
        "Building database connection pool"
    );

    let mut options = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout);

    if let Some(idle) = config.idle_timeout {
        options = options.idle_timeout(idle);
    }
    if let Some(lifetime) = config.max_lifetime {
        options = options.max_lifetime(lifetime);
    }

    let pool = options.connect(config.database_url.expose_secret()).await?;
    Ok(pool)
}

/// Constructs a PostgreSQL connection pool (`PgPool`) from environment variables.
///
/// Reads:
/// - `DATABASE_URL` (Required)
/// - `MAX_CONNECTIONS` (Optional, defaults to `"5"`)
/// - `MIN_CONNECTIONS` (Optional, defaults to `"1"`)
/// - `DB_ACQUIRE_TIMEOUT_SECS` (Optional, defaults to `"30"`)
///
/// For programmatic configuration, use [`build_pool_with_config`] and [`DbConfig`].
///
/// # Errors
///
/// Returns [`DbError::Configuration`] if `DATABASE_URL` is missing or limits are invalid,
/// or [`DbError::Sqlx`] if connecting to PostgreSQL fails.
pub async fn build_pool() -> Result<PgPool, DbError> {
    let config = DbConfig::from_env()?;
    build_pool_with_config(&config).await
}
