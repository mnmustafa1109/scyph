//! Notification traits and message data structures.

use scyph_core::AppError;
use std::collections::HashMap;
use uuid::Uuid;

/// Email message payload for dispatch via [`EmailService`].
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// Recipient email addresses.
    pub to: Vec<String>,
    /// Subject line of the email.
    pub subject: String,
    /// HTML formatted body content.
    pub html: String,
    /// Optional plaintext body fallback.
    pub text: Option<String>,
}

/// Abstract contract for asynchronous email delivery services.
pub trait EmailService: Send + Sync + 'static {
    /// Sends an [`EmailMessage`] asynchronously.
    fn send(&self, msg: EmailMessage) -> impl Future<Output = Result<(), AppError>> + Send;
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
    ) -> impl Future<Output = Result<(), AppError>> + Send;
}

/// In-app notification record structure.
#[derive(Debug, Clone)]
pub struct InAppNotification {
    /// Unique user identifier receiving the notification.
    pub user_id: String,
    /// Notification title.
    pub title: String,
    /// Notification body content.
    pub body: String,
    /// Notification category (e.g., `"alert"`, `"system"`, `"message"`).
    pub category: String,
    /// Optional structured JSON metadata context.
    pub metadata: Option<serde_json::Value>,
}

/// Abstract repository contract for storing and querying in-app user notifications.
pub trait NotificationRepository: Send + Sync + 'static {
    /// Creates a new [`InAppNotification`] record in the repository.
    fn create(&self, n: InAppNotification) -> impl Future<Output = Result<(), AppError>> + Send;
    /// Marks an in-app notification as read for a given user.
    fn mark_read(&self, id: Uuid, user: Uuid) -> impl Future<Output = Result<(), AppError>> + Send;
    /// Marks all unread in-app notifications as read for a specific user.
    fn mark_all_read(&self, user: Uuid) -> impl Future<Output = Result<(), AppError>> + Send;
    /// Lists all unread notifications for a specified user.
    fn list_unread(
        &self,
        user: Uuid,
    ) -> impl Future<Output = Result<Vec<InAppNotification>, AppError>> + Send;
}
