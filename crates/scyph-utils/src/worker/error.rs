//! Background worker error types.

use scyph_core::AppError;

/// Errors encountered during background worker execution.
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
