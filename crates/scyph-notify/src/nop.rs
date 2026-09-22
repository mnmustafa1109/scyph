//! No-op push and email notification service implementations.
//!
//! These dummy services implement the [`PushService`] and [`EmailService`] traits with zero-cost
//! `Ok(())` stubs. They are ideal for unit testing, offline development, or feature-flagging notification logic.

use crate::{
    traits::{EmailMessage, EmailService, PushNotification, PushService},
    NotifyError,
};

/// Dummy no-op [`PushService`] implementation that silently logs/swallows push notifications.
///
/// Useful for testing, offline local development, or disabling push notifications without mutating application logic.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::{NoPushService, PushNotification, PushService};
/// use std::collections::HashMap;
///
/// async fn notify(service: &impl PushService) {
///     let notif = PushNotification {
///         token: "dummy_token".into(),
///         title: "Test".into(),
///         body: "Hello".into(),
///         image: None,
///         sound: None,
///         data: HashMap::new(),
///     };
///     service.send(notif).await.unwrap();
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// notify(&NoPushService).await;
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPushService;

impl PushService for NoPushService {
    /// Instantly returns `Ok(())` without making network requests.
    async fn send(&self, _: PushNotification) -> Result<(), NotifyError> {
        Ok(())
    }
}

/// Dummy no-op [`EmailService`] implementation that silently logs/swallows email delivery requests.
///
/// Useful for unit testing, integration tests, or local environments where no SMTP server is configured.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::{EmailMessage, EmailService, NoEmailService};
///
/// async fn send_welcome(service: &impl EmailService) {
///     let msg = EmailMessage {
///         to: vec!["user@example.com".into()],
///         subject: "Welcome".into(),
///         html: "<p>Welcome!</p>".into(),
///         text: Some("Welcome!".into()),
///     };
///     service.send(msg).await.unwrap();
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// send_welcome(&NoEmailService).await;
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEmailService;

impl EmailService for NoEmailService {
    /// Instantly returns `Ok(())` without connecting to an SMTP server.
    async fn send(&self, _: EmailMessage) -> Result<(), NotifyError> {
        Ok(())
    }
}
