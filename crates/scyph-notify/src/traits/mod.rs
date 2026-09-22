//! Notification traits and message data structures.

#[cfg(all(feature = "email", feature = "fcm"))]
/// Multi-channel composite notification traits and broadcaster.
pub mod composite;

/// Email notification traits and models.
pub mod email;

/// Push notification traits and models.
pub mod push;

/// In-app notification repository traits and models.
pub mod repository;

#[cfg(all(feature = "email", feature = "fcm"))]
pub use composite::{CompositeNotification, NotificationBroadcaster};
pub use email::{EmailMessage, EmailService, EmailTemplate};
pub use push::{PushNotification, PushService, PushTemplate};
pub use repository::{InAppNotification, NotificationRepository};
