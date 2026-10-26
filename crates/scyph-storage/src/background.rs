//! Fire-and-forget object deletion with a single retry.

use std::sync::Arc;
use std::time::Duration;

use tokio::spawn;
use tokio::time::sleep;
use tracing::warn;

use crate::traits::StorageService;

/// Deletes `key` from `storage` in the background, retrying once after 1 s on
/// failure. Fire-and-forget: errors are logged, never propagated.
///
/// No-op is not handled here — callers should guard on `Option` themselves.
///
/// This spawns one Tokio task per key. For bulk deletion of many objects, use
/// [`delete_many_with_retry`] instead to avoid task/memory spikes.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use scyph_storage::{delete_with_retry, StorageService};
///
/// fn cleanup(storage: Arc<impl StorageService>, old_key: Option<String>) {
///     if let Some(key) = old_key {
///         delete_with_retry(storage, key);
///     }
/// }
/// ```
pub fn delete_with_retry<T>(storage: Arc<T>, key: String)
where
    T: StorageService,
{
    spawn(async move {
        delete_one_with_retry(&*storage, &key).await;
    });
}

/// Deletes every key in `keys` in a single background task, retrying each once.
///
/// Use this instead of calling [`delete_with_retry`] in a loop: the latter spawns
/// one Tokio task per key, which can spike memory/tasks for bulk cleanup.
pub fn delete_many_with_retry<T, I>(storage: Arc<T>, keys: I)
where
    T: StorageService,
    I: IntoIterator<Item = String>,
{
    let keys: Vec<String> = keys.into_iter().collect();
    spawn(async move {
        for key in &keys {
            delete_one_with_retry(&*storage, key).await;
        }
    });
}

/// Shared single-key delete with one retry after 1 s. Errors are logged, never
/// propagated.
async fn delete_one_with_retry<T>(storage: &T, key: &str)
where
    T: StorageService,
{
    if let Err(err) = storage.delete(key).await {
        warn!(key = %key, error = %err, "failed to delete stored object; retrying once");
        // shortcut: a single retry after 1s, so an outage longer than
        // that still orphans the object; upgrade to a reaper or a work
        // queue if the warn above shows up in production logs.
        sleep(Duration::from_secs(1)).await;
        if let Err(err) = storage.delete(key).await {
            warn!(key = %key, error = %err, "stored object orphaned after retry");
        }
    }
}
