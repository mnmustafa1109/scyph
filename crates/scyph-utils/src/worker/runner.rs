//! Worker loop runner functions.

use crate::worker::error::WorkerError;
use std::{future::Future, time::Duration};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// Handle for controlling, canceling, and gracefully joining a running background worker task.
#[derive(Debug)]
pub struct WorkerHandle {
    name: &'static str,
    cancel_token: CancellationToken,
    join_handle: JoinHandle<()>,
}

impl WorkerHandle {
    /// Returns the static name identifier of the background worker.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Triggers the cancellation signal for the background worker without awaiting loop completion.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Returns a reference to the underlying Tokio [`CancellationToken`].
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    /// Triggers cancellation and awaits the background worker task loop until it cleanly exits.
    ///
    /// Guarantees that when this async function returns, the worker task loop has terminated and
    /// zero active worker tasks remain running.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError::TaskJoin`] if the background worker task loop panicked.
    pub async fn shutdown_and_join(self) -> Result<(), WorkerError> {
        info!(
            worker = self.name,
            "Initiating graceful worker shutdown and joining task loop"
        );
        self.cancel_token.cancel();
        self.join_handle
            .await
            .map_err(|e| WorkerError::TaskJoin(e.to_string()))
    }
}

/// Spawns a periodic background task with graceful cancellation token support.
///
/// Executes `task()` every `interval` duration on a Tokio task loop.
/// Configures [`MissedTickBehavior::Skip`] to prevent worker task thundering herds/bursts after long jobs.
/// Isolates panics inside the task function to prevent crashing the background loop.
///
/// Returns a [`WorkerHandle`] which provides [`WorkerHandle::shutdown_and_join`] for 100% clean shutdown.
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
/// let worker = spawn_worker_cancel("cleanup_job", Duration::from_secs(60), cancel_token, || async {
///     println!("Cleaning up temp files...");
/// });
///
/// // Later during app shutdown:
/// worker.shutdown_and_join().await.expect("Worker cleanly stopped");
/// # }
/// ```
pub fn spawn_worker_cancel<F, Fut>(
    name: &'static str,
    interval: Duration,
    cancel_token: CancellationToken,
    task: F,
) -> WorkerHandle
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let token = cancel_token.clone();
    let join_handle = tokio::spawn(async move {
        info!(worker = name, "Background worker loop started");
        let mut ticker = time::interval(interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!(worker = name, "Background worker received shutdown signal");
                    break;
                }
                _ = ticker.tick() => {
                    let mut task_handle = tokio::spawn(task());
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            info!(worker = name, "Cancellation signal received; awaiting in-progress worker task");
                            if let Err(err) = (&mut task_handle).await {
                                error!(worker = name, error = ?err, "Background worker task panicked on shutdown join");
                            }
                            break;
                        }
                        res = &mut task_handle => {
                            if let Err(err) = res {
                                error!(worker = name, error = ?err, "Background worker task panicked or failed to join");
                            }
                        }
                    }
                }
            }
        }
    });

    WorkerHandle {
        name,
        cancel_token: token,
        join_handle,
    }
}

/// Spawns a periodic background task using a `tokio::sync::watch` channel receiver.
///
/// An alternative to [`spawn_worker_cancel`] that integrates with `tokio::sync::watch` shutdown
/// broadcasting channels (commonly used in Axum application shutdown patterns). The worker polls
/// the `shutdown` receiver on each tick and after each task run. When `*shutdown.borrow() == true`,
/// it completes any currently running task iteration and exits cleanly.
///
/// Returns a [`JoinHandle<()>`] for awaiting the worker's exit; unlike [`WorkerHandle`], this
/// variant does not provide a `cancel()` method — shutdown is signalled externally via the
/// `watch::Sender`.
///
/// ## When to Use This vs. `spawn_worker_cancel`
///
/// | Use `spawn_worker` | Use `spawn_worker_cancel` |
/// |---|---|
/// | Already have a `watch::Receiver<bool>` for app-wide shutdown | Prefer [`CancellationToken`] or need `WorkerHandle::cancel()` |
/// | Integrating with existing watch channel broadcast | Standalone task with independent lifecycle |
///
/// # Arguments
///
/// * `name` - Static human-readable identifier logged on start and shutdown.
/// * `interval` - Duration between successive task executions.
/// * `shutdown` - Watch channel receiver; worker exits when `*borrow() == true`.
/// * `task` - Closure factory returning an async task future.
///
/// # Examples
///
/// ```rust
/// use scyph_utils::worker::spawn_worker;
/// use tokio::sync::watch;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let (shutdown_tx, shutdown_rx) = watch::channel(false);
///
/// let handle = spawn_worker("metrics_flush", Duration::from_secs(30), shutdown_rx, || async {
///     println!("Flushing metrics buffer...");
///     // flush_metrics().await;
/// });
///
/// // Later, during graceful shutdown:
/// shutdown_tx.send(true).unwrap();
/// handle.await.expect("Worker exited cleanly");
/// # }
/// ```
pub fn spawn_worker<F, Fut>(
    name: &'static str,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
    task: F,
) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        info!(worker = name, "Background worker loop started");
        let mut ticker = time::interval(interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let mut task_handle = tokio::spawn(task());
                    tokio::select! {
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                info!(worker = name, "Shutdown signal received; awaiting in-progress worker task");
                                if let Err(err) = (&mut task_handle).await {
                                    error!(worker = name, error = ?err, "Background worker task panicked on shutdown join");
                                }
                                break;
                            }
                        }
                        res = &mut task_handle => {
                            if let Err(err) = res {
                                error!(worker = name, error = ?err, "Background worker task panicked or failed to join");
                            }
                        }
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
