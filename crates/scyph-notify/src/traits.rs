//! Notification traits and message data structures.

use crate::NotifyError;
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

/// Contract for strongly-typed domain email templates.
pub trait EmailTemplate {
    /// Recipient email addresses.
    fn to(&self) -> Vec<String>;

    /// Subject line of the email.
    fn subject(&self) -> String;

    /// Name of the Tera template file (e.g. `"new_message.html"`).
    fn template_name(&self) -> &str;

    /// Template context variables serialized as JSON [`serde_json::Value`].
    fn context(&self) -> serde_json::Value;

    /// Optional plaintext body fallback.
    fn text(&self) -> Option<String> {
        None
    }
}

/// Abstract contract for asynchronous email delivery services.
pub trait EmailService: Send + Sync + 'static {
    /// Sends an [`EmailMessage`] asynchronously.
    fn send(&self, msg: EmailMessage) -> impl Future<Output = Result<(), NotifyError>> + Send;

    #[cfg(feature = "email")]
    /// Renders a strongly-typed [`EmailTemplate`] using [`TemplateEngine`](crate::template::TemplateEngine) and sends the email asynchronously.
    fn send_template<'a, E: EmailTemplate + Sync + 'a>(
        &'a self,
        engine: &'a crate::template::TemplateEngine,
        template: &'a E,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send + 'a {
        async move {
            let msg = engine.render_email(template)?;
            self.send(msg).await
        }
    }
}

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
    fn create(&self, n: InAppNotification) -> impl Future<Output = Result<(), NotifyError>> + Send;
    /// Marks an in-app notification as read for a given user.
    fn mark_as_read(
        &self,
        id: Uuid,
        user: Uuid,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send;
    /// Marks all unread in-app notifications as read for a specific user.
    fn mark_all_as_read(&self, user: Uuid) -> impl Future<Output = Result<(), NotifyError>> + Send;
    /// Lists all unread notifications for a specified user.
    fn list_unread(
        &self,
        user: Uuid,
    ) -> impl Future<Output = Result<Vec<InAppNotification>, NotifyError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nop::NoPushService;

    struct TestPush {
        token: String,
        title: String,
        body: String,
    }

    impl PushTemplate for TestPush {
        fn token(&self) -> String {
            self.token.clone()
        }
        fn title(&self) -> String {
            self.title.clone()
        }
        fn body(&self) -> String {
            self.body.clone()
        }
    }

    #[tokio::test]
    async fn test_push_template_send() {
        let push_service = NoPushService;
        let template = TestPush {
            token: "fcm_token_123".into(),
            title: "Hello".into(),
            body: "World".into(),
        };

        let res = push_service.send_template(&template).await;
        assert!(res.is_ok());
    }
}
