//! Tracing subscriber initialization and non-blocking log appender setup.

use std::env;

use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Guard holding the non-blocking log appender worker thread.
///
/// Must be retained in `main()` to ensure queued log messages are flushed to stdout on application exit.
#[must_use = "TelemetryGuard worker guard must be held in main() to ensure logs are flushed on shutdown"]
#[derive(Debug)]
pub struct TelemetryGuard(Option<WorkerGuard>);

impl TelemetryGuard {
    /// Constructs a no-op [`TelemetryGuard`].
    pub fn empty() -> Self {
        Self(None)
    }

    /// Returns `true` if this guard contains an active non-blocking log worker thread.
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
