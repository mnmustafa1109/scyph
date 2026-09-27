//! Periodic background worker loop management with panic isolation and graceful shutdown.

pub mod error;
pub mod runner;

pub use error::WorkerError;
pub use runner::{spawn_worker, spawn_worker_cancel};
