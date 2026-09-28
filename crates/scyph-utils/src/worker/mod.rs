//! Periodic background worker loop management with panic isolation and graceful shutdown.
//!
//! This module provides two flavours of background worker runners:
//!
//! ## [`spawn_worker_cancel`] — Preferred (CancellationToken-based)
//!
//! Uses a [`tokio_util::sync::CancellationToken`] that integrates natively with Axum's
//! graceful shutdown and `tokio_util::task::TaskTracker`. Returns a [`WorkerHandle`] with
//! a [`WorkerHandle::shutdown_and_join`] method for clean teardown.
//!
//! ```rust
//! use scyph_utils::worker::spawn_worker_cancel;
//! use tokio_util::sync::CancellationToken;
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let cancel = CancellationToken::new();
//! let worker = spawn_worker_cancel(
//!     "metrics_flush",
//!     Duration::from_secs(30),
//!     cancel.clone(),
//!     || async { /* flush metrics */ },
//! );
//!
//! // Gracefully shut down and wait for in-progress task:
//! worker.shutdown_and_join().await.expect("worker stopped cleanly");
//! # }
//! ```
//!
//! ## [`spawn_worker`] — Watch-Channel-based
//!
//! Uses a `tokio::sync::watch` channel for shutdown signaling. Useful when integrating with
//! existing infrastructure that uses watch channels for lifecycle management.
//!
//! ## Panic Isolation
//!
//! Each task invocation is spawned in its own `tokio::spawn` call. If a task closure panics,
//! the panic is caught at the `JoinHandle` boundary and logged via `tracing::error!`. The
//! outer worker loop continues running — a single panicking iteration does not bring down the
//! background worker or the application.
//!
//! ## Missed Tick Behavior
//!
//! All workers configure [`tokio::time::MissedTickBehavior::Skip`], which means that if a
//! task takes longer than the configured interval, the skipped ticks are **not** queued up
//! for immediate catch-up. This prevents thundering-herd bursts after a slow task or GC pause.

pub mod error;
pub mod runner;

pub use error::WorkerError;
pub use runner::{WorkerHandle, spawn_worker, spawn_worker_cancel};
