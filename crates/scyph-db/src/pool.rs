//! PostgreSQL database connection pool builder.

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
/// Returns [`sqlx::Error`] if connecting to PostgreSQL fails.
///
/// # Panics
///
/// Panics if `DATABASE_URL` is missing or if connection limits cannot be parsed into `u32`.
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
pub async fn build_pool() -> Result<PgPool, sqlx::Error> {
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let max_conn = env::var("MAX_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u32>()
        .expect("MAX_CONNECTIONS must be a valid u32");

    let min_conn = env::var("MIN_CONNECTIONS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u32>()
        .expect("MIN_CONNECTIONS must be a valid u32");

    info!(
        max_connections = max_conn,
        min_connections = min_conn,
        "Building database connection pool"
    );

    PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(min_conn)
        .connect(&url)
        .await
}
