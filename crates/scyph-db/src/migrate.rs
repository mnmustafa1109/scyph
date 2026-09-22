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

use crate::DbError;
use sqlx::PgPool;
use std::{
    env,
    path::{Path, PathBuf},
};
use tracing::info;

/// Helper function to resolve environment string from `APP_ENV` to target folder name.
///
/// Maps `"development"` (or `"dev"`, or default when unset) to `"beta"` and `"production"` (or `"prod"`) to `"prod"`.
fn resolve_env_folder(raw_env: &str) -> String {
    match raw_env.to_lowercase().as_str() {
        "development" | "dev" => "beta".to_string(),
        "production" => "prod".to_string(),
        _ => raw_env.to_string(),
    }
}

/// Resolves the actual existing environment folder path inside `base_dir`.
///
/// Checks the mapped folder name (e.g., `beta` or `prod`) first, and falls back to `raw_env` if distinct.
fn resolve_target_env_path(base_dir: &Path, raw_env: &str) -> Option<PathBuf> {
    let mapped_name = resolve_env_folder(raw_env);
    let mapped_path = base_dir.join(&mapped_name);
    if mapped_path.exists() {
        return Some(mapped_path);
    }
    let raw_path = base_dir.join(raw_env);
    if raw_path.exists() {
        return Some(raw_path);
    }
    None
}

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

    // 1. Try running root migrations directory if valid
    if let Ok(migrator) = sqlx::migrate::Migrator::new(path).await {
        info!(migrations_dir = ?path, "Running root database migrations");
        migrator.run(pool).await?;
        info!("Root database migrations completed");
    }

    // 2. Try running common subfolder (<dir>/common)
    let common_path = path.join("common");
    if common_path.exists()
        && let Ok(migrator) = sqlx::migrate::Migrator::new(common_path.as_path()).await
    {
        info!(path = ?common_path, "Running common database migrations");
        migrator.run(pool).await?;
        info!("Common database migrations completed");
    }

    // 3. Try running environment-specific subfolder (<dir>/beta or <dir>/prod)
    if let Some(env_path) = resolve_target_env_path(path, &raw_env)
        && let Ok(migrator) = sqlx::migrate::Migrator::new(env_path.as_path()).await
    {
        info!(app_env = %raw_env, path = ?env_path, "Running environment database migrations");
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

    // 1. Try running root seeds directory
    if let Ok(migrator) = sqlx::migrate::Migrator::new(path).await {
        info!(path = ?path, "Running root seed data insertion");
        migrator.run(pool).await?;
        info!("Root seed data insertion completed");
    }

    // 2. Try running common seeds (<dir>/common)
    let common_path = path.join("common");
    if common_path.exists()
        && let Ok(migrator) = sqlx::migrate::Migrator::new(common_path.as_path()).await
    {
        info!(path = ?common_path, "Running common seed data insertion");
        migrator.run(pool).await?;
        info!("Common seed data insertion completed");
    }

    // 3. Try running environment seeds (<dir>/beta or <dir>/prod)
    if let Some(env_path) = resolve_target_env_path(path, &raw_env)
        && let Ok(migrator) = sqlx::migrate::Migrator::new(env_path.as_path()).await
    {
        info!(app_env = %raw_env, path = ?env_path, "Running environment seed data insertion");
        migrator.run(pool).await?;
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
