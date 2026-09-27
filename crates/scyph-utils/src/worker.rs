//! Periodic background worker loop management with panic isolation and graceful shutdown.

use std::{future::Future, time::Duration};
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Spawns a periodic background task with graceful cancellation token support.
///
/// Executes `task()` every `interval` duration on a Tokio task loop.
/// Isolates panics inside the task function to prevent crashing the background loop.
///
/// # Arguments
///
/// * `name` - Static human-readable identifier for logging.
/// * `interval` - Delay duration between task executions.
/// * `cancel_token` - Tokio [`CancellationToken`] to trigger graceful shutdown.
/// * `task` - Closure factory returning the async task future.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::worker::spawn_worker_cancel;
/// use tokio_util::sync::CancellationToken;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let cancel_token = CancellationToken::new();
/// let handle = spawn_worker_cancel("cleanup_job", Duration::from_secs(60), cancel_token.clone(), || async {
///     println!("Cleaning up temp files...");
/// });
///
/// // Later during app shutdown:
/// cancel_token.cancel();
/// # }
/// ```
pub fn spawn_worker_cancel<F, Fut>(
    name: &'static str,
    interval: Duration,
    cancel_token: CancellationToken,
    task: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        info!(worker = name, "Background worker loop started");
        let mut ticker = time::interval(interval);
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!(worker = name, "Background worker received shutdown signal");
                    break;
                }
                _ = ticker.tick() => {
                    if let Err(err) = tokio::spawn(task()).await {
                        error!(worker = name, error = ?err, "Background worker task panicked or failed to join");
                    }
                }
            }
        }
    })
}

/// Spawns a periodic background task using a `tokio::sync::watch` channel receiver.
///
/// Compatible with watch channel receiver state checks (`*shutdown.borrow() == true`).
pub fn spawn_worker<F, Fut>(
    name: &'static str,
    interval: Duration,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    task: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        info!(worker = name, "Background worker loop started");
        let mut ticker = time::interval(interval);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(err) = tokio::spawn(task()).await {
                        error!(worker = name, error = ?err, "Background worker task panicked or failed to join");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!(worker = name, "Background worker received shutdown signal");
                        break;
                    }
                }
            }
        }
    })
}
