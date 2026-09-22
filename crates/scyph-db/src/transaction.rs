//! Database transaction lifecycle management.

use crate::error::DbError;
use sqlx::{PgPool, Postgres, Transaction};

/// Begins a new database transaction from the connection pool.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if beginning the transaction fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::{begin, commit};
/// use sqlx::PgPool;
///
/// async fn update_data(pool: &PgPool) -> Result<(), scyph_db::DbError> {
///     let mut tx = begin(pool).await?;
///     // perform database operations...
///     commit(tx).await?;
///     Ok(())
/// }
/// ```
pub async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, DbError> {
    pool.begin().await.map_err(DbError::Sqlx)
}

/// Commits an active database transaction.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if committing the transaction fails.
pub async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), DbError> {
    tx.commit().await.map_err(DbError::Sqlx)
}

/// Rolls back an active database transaction.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if rolling back the transaction fails.
pub async fn rollback(tx: Transaction<'_, Postgres>) -> Result<(), DbError> {
    tx.rollback().await.map_err(DbError::Sqlx)
}
