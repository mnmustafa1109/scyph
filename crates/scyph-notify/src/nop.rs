//! No-op push and email notification service implementations.

use crate::{
    traits::{EmailMessage, EmailService, PushNotification, PushService},
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

/// A dummy no-op [`EmailService`] implementation that silently succeeds without sending emails.
///
/// Useful for testing, development, or disabling email delivery.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEmailService;

impl EmailService for NoEmailService {
    async fn send(&self, _: EmailMessage) -> Result<(), NotifyError> {
        Ok(())
    }
}
