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
/// Migrations execute in **three cascading stages** in the following order:
///
/// 1. **Root directory** (`dir`): Any `.sql` files found directly inside `dir` are applied first.
///    These are schema-level migrations shared across all environments.
/// 2. **Common subdirectory** (`<dir>/common`): If this subdirectory exists and contains `.sql`
///    files, they are applied second. Use this for migrations that must run in every environment
///    but are distinct from root-level migrations (e.g. shared reference-data tables).
/// 3. **Environment subdirectory**: Resolved from the `APP_ENV` variable and applied last.
///    Each stage uses `sqlx::migrate::Migrator`, so migrations within each stage are applied
///    in filename-sorted order and are tracked in the `_sqlx_migrations` table to avoid
///    being rerun.
///
/// ### Environment Subdirectory Resolution
///
/// | `APP_ENV` value | Primary path | Fallback path |
/// |-----------------|-------------|---------------|
/// | `"development"` / `"dev"` / unset | `<dir>/beta/` | `<dir>/development/` |
/// | `"production"` / `"prod"` | `<dir>/prod/` | `<dir>/production/` |
/// | any other value (e.g. `"staging"`) | `<dir>/staging/` | — |
///
/// If the resolved environment path does not exist, that stage is silently skipped.
/// If `dir` itself does not exist, the entire function returns `Ok(())` immediately.
///
/// ### Cascading Guarantees
///
/// Each stage runs independently through `sqlx::migrate::Migrator`, which means each stage
/// maintains its own `_sqlx_migrations` record. Migrations applied in one stage are not
/// visible to the migrator in another stage. Design your migration file names and numbering
/// scheme accordingly to avoid confusion (e.g. prefix root migrations with `0001_`, common
/// with `1001_`, environment-specific with `2001_`).
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
/// * `dir` - Path to the migrations directory. Accepts any type implementing [`AsRef<Path>`],
///   including `&str`, `String`, and [`std::path::PathBuf`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if:
/// - `sqlx::migrate::Migrator::new()` fails to load migration files from a directory.
/// - Any migration SQL statement fails to execute (e.g. syntax error, constraint violation).
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::migrate::run_migrations_from;
/// use sqlx::PgPool;
///
/// async fn setup_custom_dir(pool: &PgPool) -> Result<(), scyph_db::DbError> {
///     // Explicitly specify a custom migrations path (e.g. in integration tests)
///     std::env::set_var("APP_ENV", "development");
///     run_migrations_from(pool, "./test-fixtures/migrations").await?;
///     Ok(())
/// }
/// ```
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
/// This is a convenience wrapper around [`run_migrations_from`] that reads the target
/// directory from the `MIGRATIONS_DIR` environment variable (defaulting to `./migrations`).
///
/// ### Cascading Behavior
///
/// Internally delegates to [`run_migrations_from`], so the same three-stage cascading
/// logic applies: root → common → environment-specific subdirectory. See that function's
/// documentation for the full resolution rules and stage ordering guarantees.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or running any migration stage fails.
///
/// # Examples
///
/// Typical application startup — run pending migrations before accepting HTTP traffic:
///
/// ```rust,ignore
/// use scyph_db::{build_pool, run_migrations};
/// use sqlx::PgPool;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // MIGRATIONS_DIR defaults to "./migrations" if not set
///     std::env::set_var("APP_ENV", "production"); // routes to ./migrations/prod/
///
///     let pool = build_pool().await?;
///     run_migrations(&pool).await?;
///
///     println!("Database ready.");
///     Ok(())
/// }
/// ```
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    let dir = env::var("MIGRATIONS_DIR").unwrap_or_else(|_| "./migrations".to_string());
    run_migrations_from(pool, dir).await
}

/// Executes database seed files from a specified directory if `RUN_SEEDS` is enabled (`"true"` or `"1"`).
///
/// Seed data is applied using `sqlx::migrate::Migrator`, which tracks applied seed files in
/// `_sqlx_migrations` and skips already-applied seeds on subsequent runs. This makes
/// `run_seeds_from` safe to call on every startup — seeds are idempotent by design.
///
/// ### Guard: `RUN_SEEDS`
///
/// If the `RUN_SEEDS` environment variable is absent, set to `"false"`, or any value other
/// than `"true"` or `"1"`, the function returns `Ok(())` immediately without touching the
/// database. This prevents accidental seed execution in production if the variable is not
/// explicitly enabled.
///
/// ### Cascading Behavior
///
/// Seed files execute in the same three-stage cascading order as migrations:
/// 1. **Root directory** (`dir`): Applied first to all environments.
/// 2. **Common subdirectory** (`<dir>/common`): Applied second if it exists.
/// 3. **Environment subdirectory**: Resolved from `APP_ENV` using the same rules as
///    [`run_migrations_from`] (see that function for the full resolution table).
///
/// Each stage is skipped silently if the directory is absent or contains no `.sql` files.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
/// * `dir` - Path to the seeds directory. Accepts any type implementing [`AsRef<Path>`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or executing seed files from any stage fails.
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
        let migrator = sqlx::migrate::Migrator::new(path).await?;
        migrator.run(pool).await?;
        info!("Root seed data insertion completed");
    }

    // 2. Run common seeds (<dir>/common) if it exists and contains SQL files
    let common_path = path.join("common");
    if common_path.exists() && has_sql_files(&common_path) {
        info!(path = ?common_path, "Running common seed data insertion");
        let migrator = sqlx::migrate::Migrator::new(common_path.as_path()).await?;
        migrator.run(pool).await?;
        info!("Common seed data insertion completed");
    }

    // 3. Run environment seeds (<dir>/beta or <dir>/prod) if it exists and contains SQL files
    if let Some(env_path) = resolve_target_env_path(path, &raw_env)
        && has_sql_files(&env_path)
    {
        info!(app_env = %raw_env, path = ?env_path, "Running environment seed data insertion");
        let migrator = sqlx::migrate::Migrator::new(env_path.as_path()).await?;
        migrator.run(pool).await?;
        info!(app_env = %raw_env, "Environment seed data insertion completed");
    }

    Ok(())
}

/// Executes database seed files using default environment variables if `RUN_SEEDS` is set to `"true"` or `"1"`.
///
/// This is a convenience wrapper around [`run_seeds_from`] that reads the base seeds
/// directory from the `SEEDS_DIR` environment variable (defaulting to `./seeds`).
///
/// ### Cascading Behavior
///
/// Internally delegates to [`run_seeds_from`], so the same three-stage cascading logic
/// applies (root → common → environment-specific). Seed files are tracked in
/// `_sqlx_migrations` and are skipped if already applied.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Migration`] if loading or executing any seed stage fails.
///
/// # Examples
///
/// Populate development data on startup (harmless in production because `RUN_SEEDS` is unset):
///
/// ```rust,ignore
/// use scyph_db::{build_pool, run_migrations, run_seeds};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // In development: RUN_SEEDS=true APP_ENV=development
///     // In production:  RUN_SEEDS unset (seeds are skipped automatically)
///
///     let pool = build_pool().await?;
///     run_migrations(&pool).await?;
///     run_seeds(&pool).await?;   // no-op unless RUN_SEEDS=true|1
///
///     Ok(())
/// }
/// ```
pub async fn run_seeds(pool: &PgPool) -> Result<(), DbError> {
    let dir = env::var("SEEDS_DIR").unwrap_or_else(|_| "./seeds".to_string());
    run_seeds_from(pool, dir).await
}
