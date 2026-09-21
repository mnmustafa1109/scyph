//! Database transaction lifecycle management.

use scyph_core::AppError;
use sqlx::{PgPool, Postgres, Transaction};

/// Begins a new database transaction from the connection pool.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`AppError::Internal`] if beginning the transaction fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::{begin, commit};
/// use sqlx::PgPool;
///
/// async fn update_data(pool: &PgPool) -> Result<(), scyph_core::AppError> {
///     let mut tx = begin(pool).await?;
///     // perform database operations...
///     commit(tx).await?;
///     Ok(())
/// }
/// ```
pub async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, AppError> {
    pool.begin()
        .await
        .map_err(|e| AppError::internal_from(e, "begin transaction failed"))
}

/// Commits an active database transaction.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if committing the transaction fails.
pub async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), AppError> {
    tx.commit()
        .await
        .map_err(|e| AppError::internal_from(e, "commit transaction failed"))
}

/// Rolls back an active database transaction.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if rolling back the transaction fails.
pub async fn rollback(tx: Transaction<'_, Postgres>) -> Result<(), AppError> {
    tx.rollback()
        .await
        .map_err(|e| AppError::internal_from(e, "rollback transaction failed"))
}
