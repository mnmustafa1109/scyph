//! Database transaction lifecycle management.
//!
//! This module provides three thin ergonomic wrappers — [`begin`], [`commit`], and [`rollback`] —
//! around `sqlx`'s `PgPool::begin()`, `Transaction::commit()`, and `Transaction::rollback()`.
//!
//! ## Design Rationale
//!
//! The wrappers exist to:
//! - Normalize error handling: all three functions return [`DbError::Sqlx`] on failure,
//!   keeping call sites consistent with the rest of the crate's error model.
//! - Improve readability: `begin(pool).await?` is more self-documenting in application
//!   code than `pool.begin().await.map_err(DbError::Sqlx)?`.
//! - Enable easy mock substitution in tests by keeping the surface area minimal.
//!
//! ## Transaction Lifecycle
//!
//! ```text
//! pool ──begin()──► Transaction
//!                       │
//!             ┌─────────┴──────────┐
//!       commit(tx)           rollback(tx)
//!             │                    │
//!           Ok(())              Ok(())
//! ```
//!
//! Both `commit` and `rollback` **consume** the `Transaction` value. If neither is called
//! before the `Transaction` is dropped, sqlx automatically rolls back the transaction,
//! ensuring no partial writes survive accidental early returns or panics.
//!
//! ## Full Workflow Example
//!
//! ```rust,ignore
//! use scyph_db::{begin, commit, rollback, DbError};
//! use sqlx::PgPool;
//!
//! /// Transfers `amount` from one account to another atomically.
//! /// Rolls back automatically if any step fails.
//! async fn transfer(pool: &PgPool, from: i64, to: i64, amount: i64) -> Result<(), DbError> {
//!     let mut tx = begin(pool).await?;
//!
//!     // Debit the source account
//!     let rows = sqlx::query!(
//!         "UPDATE accounts SET balance = balance - $1 WHERE id = $2 AND balance >= $1",
//!         amount, from
//!     )
//!     .execute(&mut *tx)
//!     .await?;
//!
//!     if rows.rows_affected() == 0 {
//!         // Insufficient funds — roll back and surface a domain error
//!         rollback(tx).await?;
//!         return Err(DbError::Configuration("Insufficient balance".into()));
//!     }
//!
//!     // Credit the destination account
//!     sqlx::query!(
//!         "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
//!         amount, to
//!     )
//!     .execute(&mut *tx)
//!     .await?;
//!
//!     // All operations succeeded — persist changes
//!     commit(tx).await?;
//!     Ok(())
//! }
//! ```

use crate::error::DbError;
use sqlx::{PgPool, Postgres, Transaction};

/// Begins a new database transaction from the connection pool.
///
/// Acquires a connection from `pool` and begins a PostgreSQL transaction with the default
/// isolation level (`READ COMMITTED`). The returned `Transaction` must be explicitly
/// [`commit`]ed or [`rollback`]ed; dropping it without doing so causes an automatic rollback.
///
/// # Arguments
///
/// * `pool` - Reference to the PostgreSQL connection pool [`PgPool`].
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if:
/// - No connection is available in the pool (pool exhausted or timeout).
/// - The underlying `BEGIN` statement fails.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_db::{begin, commit, rollback, DbError};
/// use sqlx::PgPool;
///
/// async fn update_data(pool: &PgPool) -> Result<(), DbError> {
///     let mut tx = begin(pool).await?;
///
///     match sqlx::query!("UPDATE items SET active = true WHERE id = 1")
///         .execute(&mut *tx)
///         .await
///     {
///         Ok(_) => commit(tx).await,
///         Err(e) => {
///             rollback(tx).await?;
///             Err(DbError::Sqlx(e))
///         }
///     }
/// }
/// ```
pub async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, DbError> {
    pool.begin().await.map_err(DbError::Sqlx)
}

/// Commits an active database transaction.
///
/// Sends a `COMMIT` to PostgreSQL, making all changes in the transaction permanent and
/// releasing the underlying connection back to the pool. The `tx` value is consumed —
/// it cannot be used after calling this function.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle obtained from [`begin`].
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the `COMMIT` statement fails (e.g. serialization failure
/// in higher isolation levels, or a network disconnect during the commit).
pub async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), DbError> {
    tx.commit().await.map_err(DbError::Sqlx)
}

/// Rolls back an active database transaction.
///
/// Sends a `ROLLBACK` to PostgreSQL, discarding all changes made within the transaction and
/// releasing the underlying connection back to the pool. The `tx` value is consumed —
/// it cannot be used after calling this function.
///
/// Prefer calling this explicitly over relying on the automatic rollback-on-drop behavior,
/// as explicit rollbacks surface any errors that may occur during the rollback itself.
///
/// # Arguments
///
/// * `tx` - Active PostgreSQL transaction handle obtained from [`begin`].
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] if the `ROLLBACK` statement fails.
pub async fn rollback(tx: Transaction<'_, Postgres>) -> Result<(), DbError> {
    tx.rollback().await.map_err(DbError::Sqlx)
}
