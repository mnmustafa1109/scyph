//! Database transaction lifecycle management.

use scyph_core::AppError;
use sqlx::{PgPool, Postgres, Transaction};

/// Begins a new database transaction from the connection pool.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if beginning the transaction fails.
pub async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, AppError> {
    pool.begin()
        .await
        .map_err(|e| AppError::internal_from(e, "begin transaction failed"))
}

/// Commits an active database transaction.
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
/// # Errors
///
/// Returns [`AppError::Internal`] if rolling back the transaction fails.
pub async fn rollback(tx: Transaction<'_, Postgres>) -> Result<(), AppError> {
    tx.rollback()
        .await
        .map_err(|e| AppError::internal_from(e, "rollback transaction failed"))
}
