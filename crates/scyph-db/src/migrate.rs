//! Database migrations and seed data execution helpers.
//!
//! This module provides automatic cascading execution for SQL database migrations and seed files.
//!
//! ### Directory Layout & Cascading Resolution
//!
//! Migrations and seeds are executed in three cascading stages:
//! 1. **Root Directory**: SQL files located directly in the root target folder (e.g. `./migrations/` or `./seeds/`).
//! 2. **`common/` Subfolder**: Shared SQL files located in `<dir>/common/`.
//! 3. **Environment Subfolder**: Environment-specific SQL files resolved dynamically from `APP_ENV`:
//!    - **`development`** (or `"dev"`, or default if `APP_ENV` is unset) maps to **`<dir>/beta/`** (falling back to `<dir>/development/` if `<dir>/beta/` does not exist).
//!    - **`production`** (or `"prod"`) maps to **`<dir>/prod/`** (falling back to `<dir>/production/` if `<dir>/prod/` does not exist).
//!    - Any other custom `APP_ENV` value (e.g. `"staging"`) maps directly to **`<dir>/<APP_ENV>/`**.

use crate::{
    DbError,
    util::{has_sql_files, resolve_target_env_path},
};
use sqlx::PgPool;
use std::{env, path::Path};
use tracing::info;

/// Runs SQL migrations against the database pool from a specified directory.
///
/// Executes migrations in cascading order:
/// 1. Root SQL files in `dir`.
/// 2. Common SQL files (`<dir>/common`).
/// 3. Environment-specific SQL files:
///    - If `APP_ENV` is `"development"` (or unset), executes `<dir>/beta` (or `<dir>/development`).
///    - If `APP_ENV` is `"production"`, executes `<dir>/prod` (or `<dir>/production`).
///    - Otherwise, executes `<dir>/<APP_ENV>`.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
/// * `dir` - Path to the migrations directory.
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or running migrations fails.
pub async fn run_migrations_from(pool: &PgPool, dir: impl AsRef<Path>) -> Result<(), DbError> {
    let path = dir.as_ref();
    if !path.exists() {
        info!(migrations_dir = ?path, "Migrations directory does not exist, skipping migrations");
        return Ok(());
    }

    let raw_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

    // 1. Run root migrations directory if it contains SQL files
    if has_sql_files(path) {
        info!(migrations_dir = ?path, "Running root database migrations");
        let migrator = sqlx::migrate::Migrator::new(path).await?;
        migrator.run(pool).await?;
        info!("Root database migrations completed");
    }

    // 2. Run common subfolder (<dir>/common) if it exists and contains SQL files
    let common_path = path.join("common");
    if common_path.exists() && has_sql_files(&common_path) {
        info!(path = ?common_path, "Running common database migrations");
        let migrator = sqlx::migrate::Migrator::new(common_path.as_path()).await?;
        migrator.run(pool).await?;
        info!("Common database migrations completed");
    }

    // 3. Run environment-specific subfolder (<dir>/beta or <dir>/prod) if it exists and contains SQL files
    if let Some(env_path) = resolve_target_env_path(path, &raw_env)
        && has_sql_files(&env_path)
    {
        info!(app_env = %raw_env, path = ?env_path, "Running environment database migrations");
        let migrator = sqlx::migrate::Migrator::new(env_path.as_path()).await?;
        migrator.run(pool).await?;
        info!(app_env = %raw_env, "Environment database migrations completed");
    }

    Ok(())
}

/// Runs SQL migrations against the database pool using default environment variables.
///
/// Reads the target directory from `MIGRATIONS_DIR` (defaulting to `./migrations`).
/// Environment subfolders (`beta` for development, `prod` for production) are resolved automatically.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or running migrations fails.
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
    run_migrations_from(pool, dir).await
}

/// Executes database seed files from a specified directory if `RUN_SEEDS` is enabled (`"true"` or `"1"`).
///
/// Seed data is executed in cascading order:
/// 1. Root SQL files in `dir`.
/// 2. Common SQL files (`<dir>/common`).
/// 3. Environment-specific SQL files:
///    - If `APP_ENV` is `"development"` (or unset), executes `<dir>/beta` (or `<dir>/development`).
///    - If `APP_ENV` is `"production"`, executes `<dir>/prod` (or `<dir>/production`).
///    - Otherwise, executes `<dir>/<APP_ENV>`.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
/// * `dir` - Path to the seeds directory.
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or executing seeds fails.
async fn execute_seed_directory(pool: &PgPool, dir: &Path) -> Result<(), DbError> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| DbError::Seed(format!("Failed to read seeds directory {dir:?}: {e}")))?;
    let mut sql_files = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| DbError::Seed(format!("Failed to read directory entry in {dir:?}: {e}")))?
    {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sql"))
                .unwrap_or(false)
        {
            sql_files.push(path);
        }
    }

    sql_files.sort();

    for file in sql_files {
        info!(seed_file = ?file, "Executing database seed script");
        let content = tokio::fs::read_to_string(&file)
            .await
            .map_err(|e| DbError::Seed(format!("Failed to read seed file {file:?}: {e}")))?;
        let mut tx = pool.begin().await.map_err(DbError::Sqlx)?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(content.as_str()))
            .execute(&mut *tx)
            .await
            .map_err(DbError::Sqlx)?;
        tx.commit().await.map_err(DbError::Sqlx)?;
    }

    Ok(())
}

/// Executes database seed files from a specified directory if `RUN_SEEDS` is enabled (`"true"` or `"1"`).
///
/// Unlike migrations, seeds are executed directly as raw SQL scripts within database transactions,
/// preventing collisions with the internal SQLx `_sqlx_migrations` table and allowing seeds to be re-run.
///
/// Seed data is executed in cascading order:
/// 1. Root SQL files in `dir`.
/// 2. Common SQL files (`<dir>/common`).
/// 3. Environment-specific SQL files:
///    - If `APP_ENV` is `"development"` (or unset), executes `<dir>/beta` (or `<dir>/development`).
///    - If `APP_ENV` is `"production"`, executes `<dir>/prod` (or `<dir>/production`).
///    - Otherwise, executes `<dir>/<APP_ENV>`.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
/// * `dir` - Path to the seeds directory.
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or executing seeds fails.
pub async fn run_seeds_from(pool: &PgPool, dir: impl AsRef<Path>) -> Result<(), DbError> {
    let run = env::var("RUN_SEEDS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if !run {
        info!("Skipping seed data insertion (RUN_SEEDS not set to true)");
        return Ok(());
    }

    let path = dir.as_ref();
    if !path.exists() {
        info!(seeds_dir = ?path, "Seeds directory does not exist, skipping seeds");
        return Ok(());
    }

    let raw_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    info!(app_env = %raw_env, seeds_dir = ?path, "Running seed data insertion");

    // 1. Run root seeds directory if it contains SQL files
    if has_sql_files(path) {
        info!(path = ?path, "Running root seed data insertion");
        execute_seed_directory(pool, path).await?;
        info!("Root seed data insertion completed");
    }

    // 2. Run common seeds (<dir>/common) if it exists and contains SQL files
    let common_path = path.join("common");
    if common_path.exists() && has_sql_files(&common_path) {
        info!(path = ?common_path, "Running common seed data insertion");
        execute_seed_directory(pool, &common_path).await?;
        info!("Common seed data insertion completed");
    }

    // 3. Run environment seeds (<dir>/beta or <dir>/prod) if it exists and contains SQL files
    if let Some(env_path) = resolve_target_env_path(path, &raw_env)
        && has_sql_files(&env_path)
    {
        info!(app_env = %raw_env, path = ?env_path, "Running environment seed data insertion");
        execute_seed_directory(pool, &env_path).await?;
        info!(app_env = %raw_env, "Environment seed data insertion completed");
    }

    Ok(())
}

/// Executes database seed files using default environment variables if `RUN_SEEDS` is set to `"true"` or `"1"`.
///
/// Reads the base seeds directory from `SEEDS_DIR` (defaulting to `./seeds`).
/// Environment subfolders (`beta` for development, `prod` for production) are resolved automatically.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or executing seeds fails.
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
    let dir = env::var("SEEDS_DIR").unwrap_or_else(|_| "./seeds".to_string());
    run_seeds_from(pool, dir).await
}
