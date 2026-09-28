//! Push notification traits, domain notification templates, and asynchronous push delivery contracts.
//!
//! This module defines the building blocks for push notification delivery via Firebase Cloud Messaging
//! (or any compatible backend):
//!
//! - [`PushTemplate`]: A domain-level trait for encapsulating device token, title, body, and data payload.
//! - [`PushNotification`]: The resolved, ready-to-send push payload produced from a [`PushTemplate`].
//! - [`PushService`]: The core async delivery trait with raw send, template send, and background variants.
//!
//! ## Delivery Flow
//!
//! ```text
//! PushTemplate  ──► PushService::send_template()  ──► PushNotification  ──► PushService::send()
//! ```

use crate::NotifyError;
use std::collections::HashMap;
use std::sync::Arc;

/// Contract for strongly-typed domain push notification events.
///
/// Implement this trait on domain events (e.g., `OrderShippedPush`, `FriendRequestPush`) to encapsulate token,
/// title, body text, sound cues, image URLs, and custom data attributes cleanly.
///
/// ## Data Payload
///
/// The [`PushTemplate::data`] map is delivered as a silent data payload to the target device's
/// application background handler. Use it to pass structured context (e.g. deep-link route, entity ID)
/// that the mobile app can act on without displaying a visible notification.
///
/// ## Rich Notifications
///
/// Set [`PushTemplate::image`] to display a banner image in the notification shade on Android,
/// iOS (via notification service extension), and web push.
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
///         m.insert("route".into(), "/orders".into());
///         m
///     }
/// }
///
/// struct ChatMessagePush {
///     recipient_token: String,
///     sender_name: String,
///     room_id: String,
/// }
///
/// impl PushTemplate for ChatMessagePush {
///     fn token(&self) -> String { self.recipient_token.clone() }
///     fn title(&self) -> String { format!("New message from {}", self.sender_name) }
///     fn body(&self) -> String { "Tap to read...".into() }
///     fn sound(&self) -> Option<String> { Some("message.mp3".into()) }
///     fn data(&self) -> HashMap<String, String> {
///         let mut m = HashMap::new();
///         m.insert("room_id".into(), self.room_id.clone());
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
///
/// Construct this directly when you have a fully assembled notification. For domain event-driven
/// notifications, prefer implementing [`PushTemplate`] and using [`PushService::send_template`],
/// which constructs this struct automatically.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::PushNotification;
/// use std::collections::HashMap;
///
/// let notification = PushNotification {
///     token: "fcm_device_token_abc123".into(),
///     title: "Flash Sale!".into(),
///     body: "50% off all items for the next hour.".into(),
///     image: Some("https://cdn.example.com/sale-banner.jpg".into()),
///     sound: Some("default".into()),
///     data: {
///         let mut m = HashMap::new();
///         m.insert("campaign_id".into(), "summer_sale_2024".into());
///         m
///     },
/// };
/// ```
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
///
/// ## Choosing the Right Method
///
/// | Method | Use When |
/// |--------|----------|
/// | `send` | You have a pre-built [`PushNotification`] and need async delivery with error propagation. |
/// | `send_template` | You have a [`PushTemplate`] domain event and need error propagation. |
/// | `send_background` | Push delivery is best-effort; failures should only be logged, not returned. |
/// | `send_template_background` | Background best-effort push from a domain event. |
///
/// ## Implementing `PushService`
///
/// ```rust,ignore
/// use scyph_notify::{PushNotification, PushService, NotifyError};
///
/// struct MyFcmService { client: reqwest::Client }
///
/// impl PushService for MyFcmService {
///     async fn send(&self, notification: PushNotification) -> Result<(), NotifyError> {
///         // Deliver via FCM HTTP v1 API
///         Ok(())
///     }
/// }
/// ```
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
