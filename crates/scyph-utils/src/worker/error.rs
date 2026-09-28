//! Background worker error types.
//!
//! Defines [`WorkerError`] — failures that can occur when joining a background worker task,
//! typically indicating that the worker task loop panicked.

use scyph_core::AppError;

/// Errors encountered during background worker execution.
///
/// ## When Does This Occur?
///
/// [`WorkerError::TaskJoin`] is returned by [`WorkerHandle::shutdown_and_join`] when the
/// underlying Tokio task panicked. In normal operation with [`MissedTickBehavior::Skip`] and
/// proper error handling inside the task closure, this should never occur. If it does, the
/// error message will contain the panic payload.
///
/// Note that panics *inside* individual task iterations are caught and logged (not propagated),
/// so `TaskJoin` only surfaces if the outer worker loop itself panics.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::worker::WorkerError;
///
/// let err = WorkerError::TaskJoin("task panicked at 'index out of bounds'".into());
/// assert!(err.to_string().contains("Worker task error"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// Error joining Tokio background worker task.
    #[error("Worker task error: {0}")]
    TaskJoin(String),
}

impl From<WorkerError> for AppError {
    fn from(err: WorkerError) -> Self {
        match err {
            WorkerError::TaskJoin(msg) => AppError::internal(msg),
        }
    }
}
