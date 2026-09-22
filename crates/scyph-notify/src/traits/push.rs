//! Push notification traits, domain notification templates, and asynchronous push delivery contracts.

use crate::NotifyError;
use std::collections::HashMap;
use std::sync::Arc;

/// Contract for strongly-typed domain push notification events.
///
/// Implement this trait on domain events (e.g., `OrderShippedPush`, `FriendRequestPush`) to encapsulate token,
/// title, body text, sound cues, image URLs, and custom data attributes cleanly.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::PushTemplate;
/// use std::collections::HashMap;
///
/// struct OrderShippedPush {
///     device_token: String,
///     order_id: String,
/// }
///
/// impl PushTemplate for OrderShippedPush {
///     fn token(&self) -> String {
///         self.device_token.clone()
///     }
///
///     fn title(&self) -> String {
///         "Order Shipped!".into()
///     }
///
///     fn body(&self) -> String {
///         format!("Your order #{} is on its way.", self.order_id)
///     }
///
///     fn data(&self) -> HashMap<String, String> {
///         let mut m = HashMap::new();
///         m.insert("order_id".into(), self.order_id.clone());
///         m
///     }
/// }
/// ```
pub trait PushTemplate {
    /// Target device FCM registration token.
    fn token(&self) -> String;

    /// Notification title displayed in the device notification shade/banner.
    fn title(&self) -> String;

    /// Main notification body text content.
    fn body(&self) -> String;

    /// Optional rich banner image URL (supported on Android, iOS, and Web push).
    fn image(&self) -> Option<String> {
        None
    }

    /// Optional custom notification sound / alert tone (e.g. `"default"`, `"chime.mp3"`).
    fn sound(&self) -> Option<String> {
        None
    }

    /// Key-value payload data map delivered silently to background application handlers.
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

/// Abstract contract for push notification delivery services (e.g. Firebase Cloud Messaging).
///
/// Provides primary asynchronous push delivery ([`PushService::send`]), domain template sending ([`PushService::send_template`]),
/// and non-blocking background task execution variants ([`PushService::send_background`], [`PushService::send_template_background`]).
pub trait PushService: Send + Sync + 'static {
    /// Sends a [`PushNotification`] payload asynchronously.
    fn send(
        &self,
        notification: PushNotification,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send;

    /// Converts and sends a strongly-typed [`PushTemplate`] asynchronously.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if FCM authorization, HTTP transport, or delivery fails.
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

    /// Spawns a background Tokio task to send a push notification payload without blocking the calling handler.
    ///
    /// Any failure during background delivery is logged via `tracing::error!`.
    fn send_background(self: Arc<Self>, notification: PushNotification) {
        tokio::spawn(async move {
            if let Err(err) = self.send(notification).await {
                tracing::error!(error = %err, "Failed to send background push notification");
            }
        });
    }

    /// Spawns a background Tokio task to send a templated push notification without blocking the calling handler.
    ///
    /// Any failure during background delivery is logged via `tracing::error!`.
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
