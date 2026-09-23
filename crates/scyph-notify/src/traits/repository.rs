//! In-app notification data structures and repository trait abstractions.
//!
//! Provides the data model [`InAppNotification`] and repository trait [`NotificationRepository`]
//! for persisting, querying, and managing unread user in-app notification centers in PostgreSQL or memory.

use crate::NotifyError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// In-app notification record structure for user activity centers and inbox feeds.
///
/// Encapsulates notification identity, recipient UUID, message content, category categorization,
/// JSON metadata, read status, and audit timestamps.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::InAppNotification;
/// use chrono::Utc;
/// use uuid::Uuid;
/// use serde_json::json;
///
/// let now = Utc::now();
/// let notification = InAppNotification {
///     id: Uuid::now_v7(),
///     user_id: Uuid::now_v7(),
///     title: "Welcome to Scyph!".into(),
///     body: "Your account has been successfully created.".into(),
///     category: "system".into(),
///     metadata: Some(json!({ "onboarding_completed": true })),
///     is_read: false,
///     created_at: now,
///     updated_at: now,
/// };
///
/// assert!(!notification.is_read);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InAppNotification {
    /// Unique notification record identifier.
    pub id: Uuid,
    /// Target user identifier receiving the notification.
    pub user_id: Uuid,
    /// Notification headline / title.
    pub title: String,
    /// Main notification message body content.
    pub body: String,
    /// Notification category classifier (e.g., `"alert"`, `"system"`, `"message"`).
    pub category: String,
    /// Optional structured JSON metadata context (e.g., click actions, target route URL).
    pub metadata: Option<serde_json::Value>,
    /// Indicates whether the notification has been read by the user.
    pub is_read: bool,
    /// Notification creation timestamp in UTC.
    pub created_at: DateTime<Utc>,
    /// Notification last updated timestamp in UTC (e.g., when marked as read).
    pub updated_at: DateTime<Utc>,
}

/// Abstract repository contract for persisting and managing user in-app notification records.
///
/// Implementations can back this trait with PostgreSQL (via `sqlx`), Redis, or mock memory stores.
pub trait NotificationRepository: Send + Sync + 'static {
    /// Creates and persists a new [`InAppNotification`] record in the repository.
    ///
    /// # Arguments
    ///
    /// * `n` - The [`InAppNotification`] record to persist.
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
    /// # Returns
    ///
    /// Returns a vector of [`InAppNotification`] records matching the user ID with `is_read == false`.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if database query fails.
    fn list_unread(
        &self,
        user: Uuid,
    ) -> impl Future<Output = Result<Vec<InAppNotification>, NotifyError>> + Send;
}
