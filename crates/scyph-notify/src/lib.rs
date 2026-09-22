#![warn(missing_docs)]
//! Email (lettre/Tera), FCM push, and in-app notification services.

/// Dummy no-op notification service implementation.
pub mod nop;

#[cfg(feature = "email")]
/// Tera template engine wrapper for rendering email and message templates.
pub mod template;

/// Notification traits and data models.
pub mod traits;

#[cfg(feature = "fcm")]
/// FCM HTTP utility helpers.
pub mod util;

#[cfg(feature = "email")]
/// SMTP email delivery service implementation.
pub mod email;

#[cfg(feature = "fcm")]
/// Firebase Cloud Messaging (FCM) push notification service implementation.
pub mod fcm;

#[cfg(feature = "health")]
/// Health check extensions for notification services.
pub mod health;

/// Notification error type definitions.
pub mod error;

pub use error::NotifyError;
pub use nop::{NoEmailService, NoPushService};
pub use traits::{
    EmailMessage, EmailService, EmailTemplate, InAppNotification, NotificationRepository,
    PushNotification, PushService, PushTemplate,
};

#[cfg(feature = "email")]
pub use email::LettreSMTPService;

#[cfg(feature = "email")]
pub use template::TemplateEngine;

#[cfg(feature = "fcm")]
pub use fcm::FcmPushService;
