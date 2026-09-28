//! No-op push and email notification service implementations.
//!
//! These dummy services implement the [`PushService`] and [`EmailService`] traits with zero-cost
//! `Ok(())` stubs. They are ideal for unit testing, offline development, or feature-flagging notification logic.
//!
//! ## When to Use No-Op Services
//!
//! - **Unit tests**: Replace real SMTP/FCM services to avoid network calls and environment config.
//! - **Local development**: Run the full application stack without setting up an SMTP relay or Firebase project.
//! - **CI/CD pipelines**: Keep tests fast and hermetic.
//!
//! ## Dependency Injection Pattern
//!
//! Because handlers depend on the `EmailService`/`PushService` traits (not concrete types), swapping
//! in no-op services requires zero handler changes:
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use scyph_notify::{EmailService, NoEmailService};
//!
//! // Production
//! // let email: Arc<dyn EmailService> = Arc::new(LettreSMTPService::from_env().unwrap());
//!
//! // Tests / local dev
//! let email: Arc<dyn EmailService> = Arc::new(NoEmailService);
//! ```

use crate::{
    NotifyError,
    traits::{EmailMessage, EmailService, PushNotification, PushService},
};

/// Dummy no-op [`PushService`] implementation that silently discards push notifications.
///
/// Implements `PushService` with an instant `Ok(())` return — no network calls, no configuration
/// required. Use this during testing, local development, or any environment where push notification
/// delivery is intentionally disabled.
///
/// `NoPushService` is a zero-sized type (`Copy + Clone + Default`), so it carries no runtime overhead.
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

/// Dummy no-op [`EmailService`] implementation that silently discards email delivery requests.
///
/// Implements `EmailService` with an instant `Ok(())` return — no SMTP connection, no TLS handshake,
/// no credentials required. Use this during unit testing, integration tests, or local environments
/// where no SMTP relay is available.
///
/// `NoEmailService` is a zero-sized type (`Copy + Clone + Default`), so it carries no runtime overhead.
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
