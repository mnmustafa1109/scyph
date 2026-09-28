//! Abstract traits, message payloads, and domain template contracts for `scyph-notify`.
//!
//! This module is the central abstraction layer of the notification system. All application
//! code should depend on the traits defined here rather than concrete implementations, enabling
//! seamless swapping of delivery drivers (e.g. `LettreSMTPService` ↔ `NoEmailService`) across
//! production, staging, and test environments without modifying handler logic.
//!
//! ## Module Structure
//!
//! | Submodule | Key Types | Responsibility |
//! |-----------|-----------|----------------|
//! | [`email`](crate::traits::email) | [`EmailMessage`], [`EmailTemplate`], [`EmailService`] | Email payload, typed template contract, and async delivery trait |
//! | [`push`](crate::traits::push) | [`PushNotification`], [`PushTemplate`], [`PushService`] | Push payload, typed push event contract, and async delivery trait |
//! | [`composite`](crate::traits::composite) | [`CompositeNotification`], [`NotificationBroadcaster`] | Multi-channel event bundling and unified broadcaster |
//! | [`repository`](crate::traits::repository) | [`InAppNotification`], [`NotificationRepository`] | In-app notification data record and persistent store trait |
//!
//! ## Design Pattern: Template vs. Raw Message
//!
//! The crate provides two levels of abstraction for sending notifications:
//!
//! 1. **Raw message** (`EmailMessage` / `PushNotification`): Use when you have already assembled
//!    the final content and just need delivery. Good for simple, programmatic notifications.
//!
//! 2. **Typed domain template** (`EmailTemplate` / `PushTemplate`): Use when a specific domain
//!    event (e.g. `WelcomeEmail`, `OrderShippedPush`) needs to encapsulate its own recipient
//!    addresses, subject, template file name, and context variables. This pattern keeps template
//!    concerns inside the domain struct rather than scattered across handler code.
//!
//! ## Example: choosing between raw and template
//!
//! ```rust,ignore
//! // Raw message – already assembled HTML
//! use scyph_notify::{EmailMessage, EmailService};
//!
//! let msg = EmailMessage {
//!     to: vec!["user@example.com".into()],
//!     subject: "One-off notice".into(),
//!     html: "<p>Something happened.</p>".into(),
//!     text: None,
//! };
//! email_service.send(msg).await?;
//!
//! // Typed template – let TemplateEngine render welcome.html with context
//! use scyph_notify::{EmailTemplate, TemplateEngine};
//! use serde_json::json;
//!
//! struct WelcomeEmail { name: String, email: String }
//! impl EmailTemplate for WelcomeEmail {
//!     fn to(&self) -> Vec<String> { vec![self.email.clone()] }
//!     fn subject(&self) -> String { "Welcome!".into() }
//!     fn template_name(&self) -> &str { "welcome.html" }
//!     fn context(&self) -> serde_json::Value { json!({ "name": self.name }) }
//! }
//!
//! email_service.send_template(&engine, &WelcomeEmail { name: "Alice".into(), email: "alice@example.com".into() }).await?;
//! ```

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
