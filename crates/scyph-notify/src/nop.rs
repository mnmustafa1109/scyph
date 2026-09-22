//! No-op push notification service implementation.

use scyph_core::AppError;

use crate::traits::{PushNotification, PushService};

/// A dummy no-op [`PushService`] implementation that silently succeeds without delivering notifications.
///
/// Useful for testing, development, or disabling push notifications.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPushService;

impl PushService for NoPushService {
    async fn send(&self, _: PushNotification) -> Result<(), AppError> {
        Ok(())
    }
}
