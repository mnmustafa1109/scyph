//! In-app notification data structures and repository trait abstractions.
//!
//! Provides the data model [`InAppNotification`] and repository trait [`NotificationRepository`]
//! for persisting, querying, and managing unread user in-app notification centers in PostgreSQL or memory.

use crate::NotifyError;
use uuid::Uuid;

/// In-app notification record structure for user activity centers and inbox feeds.
#[derive(Debug, Clone)]
pub struct InAppNotification {
    /// Target user identifier receiving the notification.
    pub user_id: String,
    /// Notification headline / title.
    pub title: String,
    /// Main notification message body content.
    pub body: String,
    /// Notification category classifier (e.g., `"alert"`, `"system"`, `"message"`).
    pub category: String,
    /// Optional structured JSON metadata context (e.g., click actions, target route URL).
    pub metadata: Option<serde_json::Value>,
}

/// Abstract repository contract for persisting and managing user in-app notification records.
///
/// Implementations can back this trait with PostgreSQL (via `sqlx`), Redis, or mock memory stores.
pub trait NotificationRepository: Send + Sync + 'static {
    /// Creates and persists a new [`InAppNotification`] record in the repository.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if database insertion fails.
    fn create(&self, n: InAppNotification) -> impl Future<Output = Result<(), NotifyError>> + Send;

    /// Marks a specific in-app notification as read for a given user.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique notification record identifier ([`Uuid`]).
    /// * `user` - Target user identifier ([`Uuid`]).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if the record is not found or database update fails.
    fn mark_as_read(
        &self,
        id: Uuid,
        user: Uuid,
    ) -> impl Future<Output = Result<(), NotifyError>> + Send;

    /// Marks all unread in-app notifications as read for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user` - Target user identifier ([`Uuid`]).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if database update fails.
    fn mark_all_as_read(&self, user: Uuid) -> impl Future<Output = Result<(), NotifyError>> + Send;

    /// Lists all unread notifications for a specified user ordered by timestamp.
    ///
    /// # Arguments
    ///
    /// * `user` - Target user identifier ([`Uuid`]).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if database query fails.
    fn list_unread(
        &self,
        user: Uuid,
    ) -> impl Future<Output = Result<Vec<InAppNotification>, NotifyError>> + Send;
}
