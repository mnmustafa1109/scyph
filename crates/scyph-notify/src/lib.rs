//! # Scyph Notify
//!
//! `scyph-notify` provides multi-channel notification services for Axum backends, including email delivery (via `lettre` and `Tera`),
//! Firebase Cloud Messaging (FCM HTTP v1 push notifications), multi-channel broadcasting, and in-app notification repository abstractions.
//!
//! ## Key Features & Architecture
//!
//! - **Connection Reuse & Efficiency**: Both `LettreSMTPService` and `FcmPushService` reuse persistent underlying connections/connection pools (`AsyncSmtpTransport` and `reqwest::Client` pool) across all send invocations rather than creating new connections per message.
//! - **Non-Blocking Background Dispatch**: High-throughput applications can dispatch emails and push notifications asynchronously in background Tokio tasks using `send_background`, `send_template_background`, or `broadcast_background`.
//! - **Template Engine (`TemplateEngine`)**: Seamlessly compiles HTML and plaintext Tera templates, supporting strongly-typed domain templates ([`EmailTemplate`]) and automatic fallback to matching `.txt` plaintext files.
//! - **Multi-Channel Broadcaster (`NotificationBroadcaster`)**: Allows single-line event dispatch across Email and Push channels simultaneously via [`CompositeNotification`].
//! - **Automated Health Checks**: Extends services with `check_health` pings integrated into `scyph-health` registries without taking down `/readyz` API endpoints during transient external vendor outages.
//! - **No-Op Test Drivers**: Includes `NoEmailService` and `NoPushService` for zero-setup unit testing and local development.
//!
//! ## Feature Flags
//!
//! - `email`: Enables `lettre` SMTP email transport and `Tera` template rendering engine.
//! - `fcm`: Enables Firebase Cloud Messaging HTTP v1 push notification client via `reqwest` and `gcp_auth`.
//! - `health`: Enables `DbHealthExt`-style health check integration with `scyph-health`.

#![warn(missing_docs)]

/// Dummy no-op notification service implementation for unit testing and local development.
pub mod nop;

#[cfg(feature = "email")]
/// Tera template engine wrapper for compiling and rendering HTML/text email templates.
pub mod template;

/// Abstract traits and data structures for email, push, composite broadcasting, and in-app notifications.
pub mod traits;

#[cfg(feature = "fcm")]
/// Helper utilities for Firebase Cloud Messaging (FCM) v1 REST API HTTP requests.
pub mod util;

#[cfg(feature = "email")]
/// `lettre` SMTP email delivery service implementation.
pub mod email;

#[cfg(feature = "fcm")]
/// Firebase Cloud Messaging (FCM) HTTP v1 push notification service implementation.
pub mod fcm;

#[cfg(feature = "health")]
/// Health check extensions for monitoring SMTP and FCM notification services.
pub mod health;

/// Error types for notification operations.
pub mod error;

pub use error::NotifyError;
pub use nop::{NoEmailService, NoPushService};
pub use traits::{
    EmailMessage, EmailService, EmailTemplate, InAppNotification, NotificationRepository,
    PushNotification, PushService, PushTemplate,
};

#[cfg(all(feature = "email", feature = "fcm"))]
pub use traits::{CompositeNotification, NotificationBroadcaster};

#[cfg(feature = "email")]
pub use email::{LettreSMTPService, SmtpConfig};

#[cfg(feature = "email")]
pub use template::TemplateEngine;

#[cfg(feature = "fcm")]
pub use fcm::FcmPushService;
