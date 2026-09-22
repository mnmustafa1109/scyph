//! Push notification traits and data models.

use crate::NotifyError;
use std::collections::HashMap;
use std::sync::Arc;

/// Contract for strongly-typed domain push notification events.
pub trait PushTemplate {
    /// Target device FCM token.
    fn token(&self) -> String;

    /// Notification title.
    fn title(&self) -> String;

    /// Main notification body text.
    fn body(&self) -> String;

    /// Optional image URL for rich notification banners (Android / iOS / Web).
    fn image(&self) -> Option<String> {
        None
    }

    /// Optional custom notification sound / chime (e.g. `"default"`).
    fn sound(&self) -> Option<String> {
        None
    }

    /// Key-value payload data map.
    fn data(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Push notification payload for delivery via [`PushService`].
#[derive(Debug, Clone)]
pub struct PushNotification {
    /// Target device FCM token.
    pub token: String,
    /// Notification title.
    pub title: String,
    /// Main notification body text.
    pub body: String,
    /// Optional image URL for rich notification banners (Android / iOS / Web).
    pub image: Option<String>,
    /// Optional custom notification sound / chime (e.g. `"default"`, `"chime.mp3"`).
    pub sound: Option<String>,
    /// Additional custom key-value payload map.
    pub data: HashMap<String, String>,
}

/// Abstract contract for push notification delivery services.
pub trait PushService: Send + Sync + 'static {
    /// Sends a [`PushNotification`] asynchronously.
    fn send(
        &self,
        notification: PushNotification,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send;

    /// Sends a strongly-typed [`PushTemplate`] asynchronously.
    fn send_template<'a, P: PushTemplate + Sync + 'a>(
        &'a self,
        template: &'a P,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send + 'a {
        async move {
            let notification = PushNotification {
                token: template.token(),
                title: template.title(),
                body: template.body(),
                image: template.image(),
                sound: template.sound(),
                data: template.data(),
            };
            self.send(notification).await
        }
    }

    /// Spawns a background Tokio task to send the push notification asynchronously without blocking the caller.
    fn send_background(self: Arc<Self>, notification: PushNotification) {
        tokio::spawn(async move {
            if let Err(err) = self.send(notification).await {
                tracing::error!(error = %err, "Failed to send background push notification");
            }
        });
    }

    /// Spawns a background Tokio task to send a templated push notification asynchronously without blocking the caller.
    fn send_template_background<P: PushTemplate + Send + Sync + 'static>(
        self: Arc<Self>,
        template: P,
    ) {
        tokio::spawn(async move {
            if let Err(err) = self.send_template(&template).await {
                tracing::error!(error = %err, "Failed to send background templated push notification");
            }
        });
    }
}
