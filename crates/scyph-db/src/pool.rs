//! PostgreSQL database connection pool builder.
//!
//! This module provides [`build_pool`], a single async function that reads pool configuration
//! from environment variables and establishes a live `sqlx::PgPool`. The `DATABASE_URL` is
//! stored in a [`secrecy::SecretString`] to prevent accidental logging of credentials.

use crate::DbError;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use tracing::info;

/// Constructs a PostgreSQL connection pool (`PgPool`) from environment variables.
///
/// Reads the following environment variables to configure the pool:
///
/// | Variable | Required | Default | Description |
/// |----------|----------|---------|-------------|
/// | `DATABASE_URL` | **Yes** | — | Full PostgreSQL connection string (`postgres://user:pass@host/db`) |
/// | `MAX_CONNECTIONS` | No | `"5"` | Maximum number of idle + active connections in the pool |
/// | `MIN_CONNECTIONS` | No | `"1"` | Minimum number of connections maintained even during idle periods |
///
/// ## Implementation Notes
///
/// - The `DATABASE_URL` is wrapped in [`secrecy::SecretString`] before being passed to sqlx,
///   which prevents the connection string (including passwords) from appearing in debug logs,
///   `Display` output, or tracing spans.
/// - Connection pool limits are parsed as `u32`; non-numeric values cause an immediate
///   [`DbError::Configuration`] error rather than silently defaulting.
/// - Pool creation blocks until at least `MIN_CONNECTIONS` connections are established,
///   so the returned `PgPool` is guaranteed to be connected and ready to use.
///
/// # Errors
///
/// Returns [`DbError::Configuration`] if:
/// - `DATABASE_URL` is not set in the environment.
/// - `MAX_CONNECTIONS` or `MIN_CONNECTIONS` are set but cannot be parsed as `u32`.
///
/// Returns [`DbError::Sqlx`] if the actual TCP connection to PostgreSQL fails (e.g. wrong
/// host, bad credentials, firewall blocking the port).
///
/// # Examples
///
/// Basic usage with all three variables configured before calling `build_pool`:
///
/// ```rust,ignore
/// use scyph_db::build_pool;
///
/// #[tokio::main]
/// async fn main() {
///     // Set environment variables (normally done via .env file or container config)
///     std::env::set_var("DATABASE_URL", "postgres://postgres:secret@localhost:5432/mydb");
///     std::env::set_var("MAX_CONNECTIONS", "10");
///     std::env::set_var("MIN_CONNECTIONS", "2");
///
///     let pool = build_pool().await.expect("Failed to build connection pool");
///
///     // Pool is ready — use it for queries or pass it to your application state
///     println!("Pool size: {}", pool.size());
/// }
/// ```
///
/// Typical use inside an Axum application startup:
///
/// ```rust,ignore
/// use scyph_db::{build_pool, run_migrations};
/// use axum::Router;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // DATABASE_URL loaded from environment or .env via dotenvy
///     let pool = build_pool().await?;
///
///     // Run pending migrations before accepting traffic
///     run_migrations(&pool).await?;
///
///     let app = Router::new()
///         // .route(...)
///         .with_state(pool);
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
///     axum::serve(listener, app).await?;
///     Ok(())
/// }
/// ```
pub async fn build_pool() -> Result<PgPool, DbError> {
    let url = SecretString::from(env::var("DATABASE_URL").map_err(|_| {
        DbError::Configuration("DATABASE_URL environment variable must be set".into())
    })?);

    let max_conn = env::var("MAX_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u32>()
        .map_err(|e| DbError::Configuration(format!("MAX_CONNECTIONS must be a valid u32: {e}")))?;

    let min_conn = env::var("MIN_CONNECTIONS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u32>()
        .map_err(|e| DbError::Configuration(format!("MIN_CONNECTIONS must be a valid u32: {e}")))?;

    info!(
        max_connections = max_conn,
        min_connections = min_conn,
        "Building database connection pool"
    );

    let pool = PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(min_conn)
        .connect(url.expose_secret())
        .await?;

    Ok(pool)
}
