//! Abstract traits, message payloads, and domain template contracts for `scyph-notify`.
//!
//! Submodules define the core abstraction boundaries:
//! - [`email`](crate::traits::email): [`EmailMessage`] payload, [`EmailTemplate`] domain contract, and [`EmailService`] delivery trait.
//! - [`push`](crate::traits::push): [`PushNotification`] payload, [`PushTemplate`] domain contract, and [`PushService`] delivery trait.
//! - [`composite`](crate::traits::composite): Multi-channel [`CompositeNotification`] and [`NotificationBroadcaster`].
//! - [`repository`](crate::traits::repository): [`InAppNotification`] data record and [`NotificationRepository`] persistent store trait.

#[cfg(all(feature = "email", feature = "fcm"))]
/// Multi-channel composite notification traits and broadcaster.
pub mod composite;

/// Email notification traits, domain template abstractions, and asynchronous message delivery contracts.
pub mod email;

/// Push notification traits, domain notification templates, and asynchronous push delivery contracts.
pub mod push;

/// In-app notification data structures and repository trait abstractions.
pub mod repository;

#[cfg(all(feature = "email", feature = "fcm"))]
pub use composite::{CompositeNotification, NotificationBroadcaster};
pub use email::{EmailMessage, EmailService, EmailTemplate};
pub use push::{PushNotification, PushService, PushTemplate};
pub use repository::{InAppNotification, NotificationRepository};
