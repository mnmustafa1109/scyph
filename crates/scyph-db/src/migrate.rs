use crate::DbError;
use sqlx::PgPool;
use std::env;
use tracing::info;

/// Runs SQL migrations against the database pool.
///
/// Reads the target directory from the `MIGRATIONS_DIR` environment variable,
/// defaulting to `./migrations`. If the directory does not exist, migration execution is skipped gracefully.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading migrations or running them fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::run_migrations;
/// use sqlx::PgPool;
///
/// async fn init_db(pool: &PgPool) {
///     run_migrations(pool).await.expect("Migrations failed");
/// }
/// ```
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    let dir = env::var("MIGRATIONS_DIR").unwrap_or_else(|_| "./migrations".to_string());
    let path = std::path::Path::new(&dir);

    if !path.exists() {
        info!(migrations_dir = %dir, "Migrations directory does not exist, skipping migrations");
        return Ok(());
    }

    info!(migrations_dir = %dir, "Running database migrations");
    let migrator = sqlx::migrate::Migrator::new(path).await?;

    migrator.run(pool).await?;
    info!("Migrations completed successfully");
    Ok(())
}

/// Executes database seed files if the environment variable `RUN_SEEDS` is set to `"true"` or `"1"`.
///
/// Reads the base seeds directory from `SEEDS_DIR` (defaulting to `./seeds`).
/// Executes common seed data (`<SEEDS_DIR>/common`) and environment-specific seeds (`<SEEDS_DIR>/<APP_ENV>`)
/// based on the `APP_ENV` environment variable (e.g. `"prod"`, `"beta"`, `"development"`).
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading seeds or executing them fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::run_seeds;
/// use sqlx::PgPool;
///
/// async fn seed_db(pool: &PgPool) {
///     run_seeds(pool).await.expect("Seeds failed");
/// }
/// ```
pub async fn run_seeds(pool: &PgPool) -> Result<(), DbError> {
    let run = env::var("RUN_SEEDS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !run {
        info!("Skipping seed data insertion (RUN_SEEDS not set to true)");
        return Ok(());
    }

    let seeds_base = env::var("SEEDS_DIR").unwrap_or_else(|_| "./seeds".to_string());
    let env_name = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    info!(app_env = %env_name, seeds_dir = %seeds_base, "Running seed data insertion for environment");

    let common_path = std::path::Path::new(&seeds_base).join("common");
    if common_path.exists() {
        let migrator = sqlx::migrate::Migrator::new(common_path).await?;
        migrator.run(pool).await?;
        info!("Common seed data insertion completed");
    }

    let env_path = std::path::Path::new(&seeds_base).join(&env_name);
    if env_path.exists() {
        let migrator = sqlx::migrate::Migrator::new(env_path).await?;
        migrator.run(pool).await?;
        info!(app_env = %env_name, "Environment seed data insertion completed");
    } else {
        info!(path = ?env_path, "No specific seed data directory found for environment: {}", env_name);
    }

    Ok(())
}
