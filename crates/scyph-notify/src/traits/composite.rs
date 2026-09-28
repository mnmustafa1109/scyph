//! Multi-channel composite notification traits and broadcaster.
//!
//! Provides the [`CompositeNotification`] trait for bundling multiple communication channels (Email + Push)
//! into a single domain event, and [`NotificationBroadcaster`] for dispatching composite events sequentially or in background Tokio tasks.
//!
//! ## Architecture
//!
//! A [`CompositeNotification`] implementation represents a single business event (e.g. "User Registered",
//! "Order Shipped") that should fan out across multiple channels. Each channel is optional — returning
//! `None` from `email()` or `push()` skips that channel silently.
//!
//! ```text
//! UserRegisteredEvent (CompositeNotification)
//!     ├── email() → Some(WelcomeEmail)  ──► EmailService
//!     └── push()  → Some(WelcomePush)   ──► PushService
//! ```
//!
//! ## Injecting `NotificationBroadcaster` as Axum State
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use scyph_notify::{NotificationBroadcaster, LettreSMTPService, FcmPushService, TemplateEngine};
//!
//! async fn build_app() {
//!     let email_svc = Arc::new(LettreSMTPService::from_env().unwrap());
//!     let push_svc = Arc::new(FcmPushService::from_env().await.unwrap());
//!     let engine = Arc::new(TemplateEngine::new("templates/").unwrap());
//!
//!     let broadcaster = Arc::new(NotificationBroadcaster::new(
//!         email_svc, push_svc, engine,
//!     ));
//!
//!     let app = axum::Router::new()
//!         .route("/register", axum::routing::post(register_handler))
//!         .layer(axum::extract::Extension(broadcaster));
//! }
//! ```

use super::{email::EmailService, email::EmailTemplate, push::PushService, push::PushTemplate};
use crate::{NotifyError, template::TemplateEngine};
use std::sync::Arc;

/// Abstract contract for multi-channel domain notifications bundling Email and Push templates.
///
/// Types implementing this trait define associated [`EmailTemplate`] and [`PushTemplate`] types.
/// Returning `Some(...)` for `email()` or `push()` causes [`NotificationBroadcaster`] to dispatch across that channel.
/// Returning `None` (the default) silently skips that channel.
///
/// ## When to Use
///
/// Use `CompositeNotification` when a single business event triggers notifications across multiple
/// channels simultaneously. This keeps all notification concerns co-located in one domain type
/// rather than scattered across handler code.
///
/// ## Partial Channels
///
/// You do not need to provide both channels. A notification that only sends email can leave the
/// default `push()` implementation (returns `None`), and vice versa.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_notify::{CompositeNotification, EmailTemplate, PushTemplate};
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// #[derive(Clone)]
/// struct WelcomeEmailTemplate { email: String, name: String }
/// impl EmailTemplate for WelcomeEmailTemplate {
///     fn to(&self) -> Vec<String> { vec![self.email.clone()] }
///     fn subject(&self) -> String { "Welcome!".into() }
///     fn template_name(&self) -> &str { "welcome.html" }
///     fn context(&self) -> serde_json::Value { json!({ "name": self.name }) }
/// }
///
/// #[derive(Clone)]
/// struct WelcomePushTemplate { token: String, name: String }
/// impl PushTemplate for WelcomePushTemplate {
///     fn token(&self) -> String { self.token.clone() }
///     fn title(&self) -> String { "Welcome! 🎉".into() }
///     fn body(&self) -> String { format!("Hi {}, your account is ready.", self.name) }
/// }
///
/// struct UserRegisteredEvent {
///     email: WelcomeEmailTemplate,
///     push: WelcomePushTemplate,
/// }
///
/// impl CompositeNotification for UserRegisteredEvent {
///     type Email = WelcomeEmailTemplate;
///     type Push = WelcomePushTemplate;
///
///     fn email(&self) -> Option<Self::Email> {
///         Some(self.email.clone())
///     }
///
///     fn push(&self) -> Option<Self::Push> {
///         Some(self.push.clone())
///     }
/// }
///
/// // In an Axum handler:
/// // broadcaster.broadcast_background(UserRegisteredEvent { email, push });
/// ```
pub trait CompositeNotification {
    /// Associated email template type implementing [`EmailTemplate`].
    type Email: EmailTemplate + Send + Sync;
    /// Associated push notification template type implementing [`PushTemplate`].
    type Push: PushTemplate + Send + Sync;

    /// Returns the optional email template payload for this notification.
    fn email(&self) -> Option<Self::Email> {
        None
    }

    /// Returns the optional push notification template payload for this notification.
    fn push(&self) -> Option<Self::Push> {
        None
    }
}

/// Unified multi-channel notification dispatcher for broadcasting events across Email and Push services.
///
/// Bundles shared [`EmailService`], [`PushService`], and [`TemplateEngine`] references.
/// Applications can inject `NotificationBroadcaster` as Axum state for single-line multi-channel dispatch.
///
/// ## Thread Safety
///
/// `NotificationBroadcaster` implements [`Clone`] and wraps all services in [`Arc`], making it safe
/// to share across Axum handlers and background tasks.
///
/// ## Synchronous vs. Background Broadcasting
///
/// - Use [`broadcast`](NotificationBroadcaster::broadcast) when you need to propagate errors or
///   await delivery confirmation before responding to the client.
/// - Use [`broadcast_background`](NotificationBroadcaster::broadcast_background) for fire-and-forget
///   dispatch where notification failure should not affect the HTTP response (most common).
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use scyph_notify::{
///     NotificationBroadcaster, NoEmailService, NoPushService, TemplateEngine,
///     CompositeNotification,
/// };
///
/// # async fn example() {
/// let broadcaster = Arc::new(NotificationBroadcaster::new(
///     Arc::new(NoEmailService),
///     Arc::new(NoPushService),
///     Arc::new(TemplateEngine::new("templates/").unwrap()),
/// ));
///
/// // In a handler — fire-and-forget across all channels:
/// broadcaster.clone().broadcast_background(my_event);
///
/// // Or await delivery with error propagation:
/// broadcaster.broadcast(&my_event).await?;
/// # }
/// ```
#[derive(Clone)]
pub struct NotificationBroadcaster<E: EmailService, P: PushService> {
    email_service: Arc<E>,
    push_service: Arc<P>,
    template_engine: Arc<TemplateEngine>,
}

impl<E: EmailService, P: PushService> NotificationBroadcaster<E, P> {
    /// Constructs a new [`NotificationBroadcaster`] instance with shared service handles.
    ///
    /// # Arguments
    ///
    /// * `email_service` - Shared [`EmailService`] implementation wrapped in [`Arc`].
    /// * `push_service` - Shared [`PushService`] implementation wrapped in [`Arc`].
    /// * `template_engine` - Shared [`TemplateEngine`] wrapped in [`Arc`].
    pub fn new(
        email_service: Arc<E>,
        push_service: Arc<P>,
        template_engine: Arc<TemplateEngine>,
    ) -> Self {
        Self {
            email_service,
            push_service,
            template_engine,
        }
    }

    /// Broadcasts a [`CompositeNotification`] event across all enabled channels (Email and Push) sequentially.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if template rendering or delivery fails on any enabled channel.
    pub async fn broadcast<N: CompositeNotification>(&self, event: &N) -> Result<(), NotifyError> {
        if let Some(email) = event.email() {
            self.email_service
                .send_template(&self.template_engine, &email)
                .await?;
        }

        if let Some(push) = event.push() {
            self.push_service.send_template(&push).await?;
        }

        Ok(())
    }

    /// Broadcasts a [`CompositeNotification`] event asynchronously in a background Tokio task.
    ///
    /// Non-blocking method that immediately returns. Any error encountered during dispatch is logged via `tracing::error!`.
    pub fn broadcast_background<N: CompositeNotification + Send + Sync + 'static>(
        self: Arc<Self>,
        event: N,
    ) {
        tokio::spawn(async move {
            if let Err(err) = self.broadcast(&event).await {
                tracing::error!(error = %err, "Failed to broadcast multi-channel notification");
            }
        });
    }
}
