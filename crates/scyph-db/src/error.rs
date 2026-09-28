//! Database error type definitions.
//!
//! This module defines [`DbError`], the single unified error type returned by all public
//! functions in `scyph-db`. It wraps `sqlx` errors and adds configuration-level failures
//! with human-readable context messages.
//!
//! ## Error Hierarchy
//!
//! ```text
//! DbError
//! ├── Sqlx(sqlx::Error)          — Query / connection failures
//! ├── Configuration(String)      — Missing env vars, invalid values
//! └── Migration(MigrateError)    — Migration load / execution failures
//! ```
//!
//! ## Conversion to `AppError`
//!
//! [`DbError`] implements `From<DbError> for AppError` (from `scyph-core`), so it can be
//! used directly in Axum handlers that return `Result<_, AppError>` via the `?` operator:
//!
//! ```rust,ignore
//! use scyph_db::DbError;
//! use scyph_core::AppError;
//!
//! async fn get_user(pool: &sqlx::PgPool, id: i64) -> Result<String, AppError> {
//!     let row = sqlx::query_scalar!("SELECT name FROM users WHERE id = $1", id)
//!         .fetch_one(pool)
//!         .await
//!         .map_err(DbError::Sqlx)?;  // DbError -> AppError via From impl
//!     Ok(row)
//! }
//! ```

use scyph_core::AppError;

/// Unified error type for all database operations in `scyph-db`.
///
/// All public async functions in this crate return `Result<_, DbError>`. The three variants
/// map directly to the three failure modes: infrastructure errors from `sqlx`, missing or
/// invalid configuration supplied at startup, and migration runner failures.
///
/// ## Matching on Variants
///
/// ```rust,ignore
/// use scyph_db::DbError;
///
/// async fn handle_error(err: DbError) {
///     match err {
///         DbError::Sqlx(e) => {
///             // Check for specific sqlx error kinds
///             if matches!(e, sqlx::Error::RowNotFound) {
///                 eprintln!("Record not found");
///             } else {
///                 eprintln!("Database error: {e}");
///             }
///         }
///         DbError::Configuration(msg) => {
///             eprintln!("Startup configuration error: {msg}");
///             // Typically unrecoverable — application should exit
///         }
///         DbError::Migration(e) => {
///             eprintln!("Migration failed: {e}");
///             // Check migration file syntax and version ordering
///         }
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// Failure connecting to or querying PostgreSQL via `sqlx`.
    ///
    /// Wraps any [`sqlx::Error`] produced by connection acquisition, query execution,
    /// transaction management, or pool operations. Common causes include:
    /// - Network errors or TCP timeouts reaching the PostgreSQL host.
    /// - Authentication failures (wrong password or role).
    /// - Constraint violations (`UNIQUE`, `FOREIGN KEY`, `NOT NULL`).
    /// - `sqlx::Error::RowNotFound` when `fetch_one()` returns no rows.
    /// - `sqlx::Error::PoolTimedOut` when all connections are saturated.
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Missing or invalid configuration detected before any database connection is attempted.
    ///
    /// Contains a human-readable message describing what is wrong and which environment
    /// variable or value is invalid. Returned by [`crate::build_pool`] when:
    /// - `DATABASE_URL` is not set.
    /// - `MAX_CONNECTIONS` or `MIN_CONNECTIONS` cannot be parsed as `u32`.
    ///
    /// Configuration errors are typically fatal — the application cannot start without
    /// correct environment configuration.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Migration or seed runner failure from `sqlx::migrate`.
    ///
    /// Wraps [`sqlx::migrate::MigrateError`], which is returned when:
    /// - A migration directory cannot be read or contains invalid `.sql` files.
    /// - A migration SQL statement fails to execute (e.g. syntax error, type mismatch).
    /// - The `_sqlx_migrations` tracking table cannot be created or queried.
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

impl From<DbError> for AppError {
    /// Converts any [`DbError`] into an [`AppError::internal`] error.
    ///
    /// This blanket conversion allows `DbError` to be propagated through Axum handlers
    /// and service functions that return `Result<_, AppError>` using the `?` operator.
    /// The error message from the `DbError` variant is preserved in the internal error body.
    fn from(err: DbError) -> Self {
        AppError::internal(err.to_string())
    }
}
