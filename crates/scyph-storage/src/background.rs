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
/// a worker queue (e.g. `scyph-utils` worker) instead to avoid task/memory
/// spikes.
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
        if let Err(err) = storage.delete(&key).await {
            warn!(key = %key, error = %err, "failed to delete stored object; retrying once");
            // shortcut: a single retry after 1s, so an outage longer than
            // that still orphans the object; upgrade to a reaper or a work
            // queue if the warn above shows up in production logs.
            sleep(Duration::from_secs(1)).await;
            if let Err(err) = storage.delete(&key).await {
                warn!(key = %key, error = %err, "stored object orphaned after retry");
            }
        }
    });
}
