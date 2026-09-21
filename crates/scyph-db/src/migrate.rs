//! Database migrations and seed data execution helpers.

use sqlx::PgPool;
use std::env;
use tracing::info;

/// Runs compiled SQL migrations from the `./migrations` directory against the database pool.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Panics
///
/// Panics if migration execution fails against the database.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::run_migrations;
/// use sqlx::PgPool;
///
/// async fn init_db(pool: &PgPool) {
///     run_migrations(pool).await;
/// }
/// ```
pub async fn run_migrations(pool: &PgPool) {
    info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run database migrations");
    info!("Migrations completed successfully");
}

/// Executes database seed files if the environment variable `RUN_SEEDS` is set to `"true"` or `"1"`.
///
/// Executes common seed data (`./seeds/common`) and environment-specific seeds (`./seeds/prod` or `./seeds/beta`)
/// based on the `APP_ENV` environment variable.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Panics
///
/// Panics if seed execution fails against the database.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::run_seeds;
/// use sqlx::PgPool;
///
/// async fn seed_db(pool: &PgPool) {
///     run_seeds(pool).await;
/// }
/// ```
pub async fn run_seeds(pool: &PgPool) {
    let run = std::env::var("RUN_SEEDS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !run {
        info!("Skipping seed data insertion (RUN_SEEDS not set to true)");
        return;
    }

    let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    info!(app_env = %env, "Running seed data insertion for environment");

    sqlx::migrate!("./seeds/common")
        .run(pool)
        .await
        .expect("Failed to run common seed data insertion");

    match env.as_str() {
        "prod" => {
            sqlx::migrate!("./seeds/prod")
                .run(pool)
                .await
                .expect("Failed to run production seed data insertion");
        }
        "beta" => {
            sqlx::migrate!("./seeds/beta")
                .run(pool)
                .await
                .expect("Failed to run staging seed data insertion");
        }
        _ => {
            info!("No specific seed data for environment: {}", env);
        }
    }
}
