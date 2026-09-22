//! Multi-channel composite notification traits and broadcaster.

use super::{email::EmailService, email::EmailTemplate, push::PushService, push::PushTemplate};
use crate::{template::TemplateEngine, NotifyError};
use std::sync::Arc;

/// Abstract contract for multi-channel domain notifications (Email + Push).
pub trait CompositeNotification {
    /// Associated email template type.
    type Email: EmailTemplate + Send + Sync;
    /// Associated push notification template type.
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
#[derive(Clone)]
pub struct NotificationBroadcaster<E: EmailService, P: PushService> {
    email_service: Arc<E>,
    push_service: Arc<P>,
    template_engine: Arc<TemplateEngine>,
}

impl<E: EmailService, P: PushService> NotificationBroadcaster<E, P> {
    /// Constructs a new [`NotificationBroadcaster`] instance.
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

    /// Broadcasts a [`CompositeNotification`] event across all configured channels (Email and Push).
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

    /// Broadcasts a [`CompositeNotification`] event in the background via a Tokio task.
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
