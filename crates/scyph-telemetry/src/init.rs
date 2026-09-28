//! Tracing subscriber initialization and non-blocking log appender setup.
//!
//! This module provides [`init_tracing`], which sets up the global `tracing` subscriber
//! with a non-blocking stdout writer, and [`TelemetryGuard`], which keeps the background
//! writer thread alive for the duration of the application.
//!
//! ## Non-Blocking Architecture
//!
//! `tracing_appender::non_blocking` spawns a dedicated background thread that owns the
//! actual stdout writer. Log records are sent to this thread via an in-memory channel,
//! allowing Tokio worker threads to return from log calls immediately without waiting for
//! I/O to complete.
//!
//! The `WorkerGuard` returned by `non_blocking` keeps the background thread alive. When the
//! guard is dropped, the thread is signaled to flush all queued messages and then terminate.
//! This is why the guard **must be held in `main()`** — dropping it early truncates logs.
//!
//! ## Log Formats
//!
//! | `LOG_FORMAT` value | Output style | Best for |
//! |--------------------|-------------|----------|
//! | `"json"` (default) | Newline-delimited JSON objects | Production, log aggregators (Datadog, CloudWatch, Loki) |
//! | `"pretty"` | Human-readable multi-line ANSI output | Local development, debugging |
//!
//! JSON format example:
//! ```json
//! {"timestamp":"2026-09-28T12:00:00Z","level":"INFO","fields":{"message":"Tracing initialized","format":"json"},"target":"scyph_telemetry::init"}
//! ```
//!
//! Pretty format example:
//! ```text
//!   2026-09-28T12:00:00Z  INFO scyph_telemetry::init: Tracing initialized format="json"
//! ```

use std::env;

use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Guard holding the non-blocking log appender worker thread.
///
/// This struct wraps a `tracing_appender::non_blocking::WorkerGuard`. While this guard is
/// alive, the background logging thread continues to drain the log record channel and write
/// to stdout. When the guard is dropped, the background thread flushes all remaining buffered
/// log records and then shuts down cleanly.
///
/// ## Lifetime Requirements
///
/// **Must be held in `main()`** for the entire duration of the application:
///
/// ```rust,ignore
/// #[tokio::main]
/// async fn main() {
///     // Assign to a named binding — NOT `let _ = ...` (that drops immediately!)
///     let _guard = init_tracing().expect("Tracing initialized");
///
///     // All log records emitted after this point are captured
///     run_application().await;
///
///     // _guard is dropped here, flushing any remaining buffered log records
/// }
/// ```
///
/// Using `let _ = init_tracing()` (with a wildcard binding) drops the guard immediately,
/// causing the background thread to exit and all subsequent log records to be silently
/// discarded.
///
/// ## Empty Guard
///
/// [`TelemetryGuard::empty`] returns a no-op guard with `None` as the inner `WorkerGuard`.
/// This is returned when the subscriber is already initialized (double-init scenario, common
/// in integration tests). The `#[must_use]` attribute ensures the guard is not accidentally
/// discarded when the variant is active.
#[must_use = "TelemetryGuard worker guard must be held in main() to ensure logs are flushed on shutdown"]
#[derive(Debug)]
pub struct TelemetryGuard(Option<WorkerGuard>);

impl TelemetryGuard {
    /// Constructs a no-op [`TelemetryGuard`] with no associated worker thread.
    ///
    /// Returned by [`init_tracing`] when a global subscriber is already installed
    /// (e.g. during integration test runs that call `init_tracing` multiple times).
    /// Dropping this guard has no effect.
    pub fn empty() -> Self {
        Self(None)
    }

    /// Returns `true` if this guard contains an active non-blocking log worker thread.
    ///
    /// Returns `false` for guards created via [`TelemetryGuard::empty`].
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
}

/// Initializes global tracing subscriber with environment filtering (`RUST_LOG`) and non-blocking stdout log output.
///
/// # Environment Variables
/// - `RUST_LOG`: Configures log level filtering (default: `"info"`).
/// - `LOG_FORMAT`: Configures log output format (`"json"` or `"pretty"`, default: `"json"`).
///
/// # Errors
///
/// Returns an error string if subscriber initialization fails.
///
/// # Examples
///
/// ```rust,no_run
/// use scyph_telemetry::init::init_tracing;
///
/// fn main() {
///     let _guard = init_tracing().expect("Tracing initialized");
/// }
/// ```
pub fn init_tracing() -> Result<TelemetryGuard, String> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".into());

    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

    let res = match format.as_str() {
        "pretty" => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty().with_writer(non_blocking))
            .try_init(),
        _ => tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_writer(non_blocking))
            .try_init(),
    };

    match res {
        Ok(()) => {
            info!(format = %format, "Tracing subscriber initialized successfully");
            Ok(TelemetryGuard(Some(guard)))
        }
        Err(err) => {
            let err_str = err.to_string();
            if err_str.contains("already been set") {
                Ok(TelemetryGuard::empty())
            } else {
                Err(format!(
                    "Failed to initialize tracing subscriber: {err_str}"
                ))
            }
        }
    }
}
