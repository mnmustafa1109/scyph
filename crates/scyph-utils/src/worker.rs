// crates/scyph-utils/src/worker.rs
use std::{future::Future, time::Duration};
use tokio::time;
use tracing::{error, info};

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
        info!(worker = name, "Worker started");
        let mut ticker = time::interval(interval);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = tokio::spawn(task()).await {
                        error!(worker = name, error = ?e, "Worker task panicked");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() { info!(worker = name, "Worker shutting down"); break; }
                }
            }
        }
    })
}
