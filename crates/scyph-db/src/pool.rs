//! PostgreSQL database connection pool builder.

use crate::DbError;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use tracing::info;

/// Constructs a PostgreSQL connection pool (`PgPool`) from environment variables.
///
/// Reads:
/// - `DATABASE_URL` (Required)
/// - `MAX_CONNECTIONS` (Optional, defaults to `"5"`)
/// - `MIN_CONNECTIONS` (Optional, defaults to `"1"`)
///
/// # Errors
///
/// Returns [`DbError::Configuration`] if `DATABASE_URL` is missing or limits are invalid,
/// or [`DbError::Sqlx`] if connecting to PostgreSQL fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::build_pool;
///
/// async fn setup() {
///     std::env::set_var("DATABASE_URL", "postgres://postgres:password@localhost/mydb");
///     let pool = build_pool().await.expect("Pool created");
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
