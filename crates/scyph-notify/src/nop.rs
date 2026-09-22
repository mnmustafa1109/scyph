//! No-op push notification service implementation.

use crate::{
    traits::{PushNotification, PushService},
    NotifyError,
};

/// A dummy no-op [`PushService`] implementation that silently succeeds without delivering notifications.
///
/// Useful for testing, development, or disabling push notifications.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPushService;

impl PushService for NoPushService {
    async fn send(&self, _: PushNotification) -> Result<(), NotifyError> {
        Ok(())
    }
}
