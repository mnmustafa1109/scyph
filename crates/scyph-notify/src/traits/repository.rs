//! In-app notification data structures and repository trait.

use crate::NotifyError;
use uuid::Uuid;

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
