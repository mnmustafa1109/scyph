//! Email notification traits, domain template abstractions, and asynchronous message delivery contracts.
//!
//! This module defines the building blocks for email delivery in `scyph-notify`:
//!
//! - [`EmailMessage`]: A concrete, already-rendered email payload ready for SMTP delivery.
//! - [`EmailTemplate`]: A domain-level trait for encapsulating template name, context, and recipients.
//! - [`EmailService`]: The core async delivery trait with raw send, template send, and background variants.
//!
//! ## Delivery Flow
//!
//! ```text
//! EmailTemplate  ──► TemplateEngine::render_email()  ──► EmailMessage  ──► EmailService::send()
//! ```
//!
//! Or, using the combined helper:
//! ```text
//! EmailService::send_template(&engine, &template)
//! ```

use crate::NotifyError;
use std::sync::Arc;

/// Email message payload for dispatch via [`EmailService`].
///
/// Contains recipient addresses, subject header, HTML formatted body, and an optional plaintext alternative.
///
/// Construct this directly when you have pre-rendered email content. For template-driven emails,
/// use [`EmailTemplate`] with [`EmailService::send_template`] instead — the [`crate::TemplateEngine`]
/// will produce an `EmailMessage` internally from the template.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::EmailMessage;
///
/// let msg = EmailMessage {
///     to: vec!["alice@example.com".into(), "bob@example.com".into()],
///     subject: "Your order has shipped".into(),
///     html: "<h1>Order #1234</h1><p>Your package is on the way!</p>".into(),
///     text: Some("Order #1234 - Your package is on the way!".into()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// List of recipient email addresses.
    pub to: Vec<String>,
    /// Subject line of the email message.
    pub subject: String,
    /// HTML formatted body content.
    pub html: String,
    /// Optional plaintext body fallback for legacy email clients.
    pub text: Option<String>,
}

/// Contract for strongly-typed domain email templates.
///
/// Implement this trait on domain structs (e.g. `WelcomeEmail`, `PasswordResetEmail`) to encapsulate template selection,
/// context variables, subject headers, and recipient target addresses cleanly.
///
/// ## Plaintext Fallback Behavior
///
/// When [`EmailTemplate::template_name`] ends in `.html` and [`EmailTemplate::text`] returns `None`,
/// the [`crate::TemplateEngine`] will automatically attempt to render a sibling `.txt` template
/// with the same base name (e.g. `welcome.txt` alongside `welcome.html`). If no `.txt` file exists,
/// the email is sent HTML-only.
///
/// ## Context Variables
///
/// The [`EmailTemplate::context`] method returns a [`serde_json::Value`] (typically a JSON object)
/// that is passed directly into the Tera rendering context. Every key in the object becomes a
/// template variable: `{{ name }}`, `{{ reset_url }}`, etc.
///
/// # Examples
///
/// ```rust
/// use scyph_notify::EmailTemplate;
/// use serde_json::json;
///
/// struct WelcomeEmail {
///     user_email: String,
///     user_name: String,
/// }
///
/// impl EmailTemplate for WelcomeEmail {
///     fn to(&self) -> Vec<String> {
///         vec![self.user_email.clone()]
///     }
///
///     fn subject(&self) -> String {
///         "Welcome to Scyph!".into()
///     }
///
///     fn template_name(&self) -> &str {
///         // TemplateEngine will also try to render "welcome.txt" for plaintext fallback
///         "welcome.html"
///     }
///
///     fn context(&self) -> serde_json::Value {
///         json!({ "name": self.user_name })
///     }
/// }
///
/// struct PasswordResetEmail {
///     user_email: String,
///     reset_token: String,
/// }
///
/// impl EmailTemplate for PasswordResetEmail {
///     fn to(&self) -> Vec<String> { vec![self.user_email.clone()] }
///     fn subject(&self) -> String { "Reset your password".into() }
///     fn template_name(&self) -> &str { "password_reset.html" }
///     fn context(&self) -> serde_json::Value {
///         json!({
///             "reset_url": format!("https://example.com/reset?token={}", self.reset_token),
///             "expires_in": "24 hours",
///         })
///     }
///     // Override with hardcoded plaintext instead of a .txt template file
///     fn text(&self) -> Option<String> {
///         Some(format!(
///             "Reset your password: https://example.com/reset?token={}",
///             self.reset_token
///         ))
///     }
/// }
/// ```
pub trait EmailTemplate {
    /// Recipient email addresses for this templated email.
    fn to(&self) -> Vec<String>;

    /// Subject line of the email.
    fn subject(&self) -> String;

    /// Name of the Tera template file (e.g. `"welcome.html"`).
    fn template_name(&self) -> &str;

    /// Template context variables serialized as JSON [`serde_json::Value`].
    fn context(&self) -> serde_json::Value;

    /// Optional explicit plaintext body content. If `None` and `template_name()` ends in `.html`,
    /// [`TemplateEngine`](crate::TemplateEngine) will automatically attempt to render a matching `.txt` file.
    fn text(&self) -> Option<String> {
        None
    }
}

/// Abstract contract for asynchronous email delivery services.
///
/// Provides primary asynchronous delivery ([`EmailService::send`]), template rendering ([`EmailService::send_template`]),
/// and non-blocking background dispatch variants ([`EmailService::send_background`], [`EmailService::send_template_background`]).
///
/// ## Choosing the Right Method
///
/// | Method | Use When |
/// |--------|----------|
/// | `send` | You have pre-rendered [`EmailMessage`] content and need async delivery with error propagation. |
/// | `send_template` | You have an [`EmailTemplate`] and need delivery with error propagation back to the caller. |
/// | `send_background` | Delivery errors are non-critical (e.g. welcome emails). Caller does not need to await. |
/// | `send_template_background` | Template-driven background dispatch. Handler returns immediately. |
///
/// ## Implementing `EmailService`
///
/// ```rust,ignore
/// use scyph_notify::{EmailMessage, EmailService, NotifyError};
///
/// struct MySmtpService { /* ... */ }
///
/// impl EmailService for MySmtpService {
///     async fn send(&self, msg: EmailMessage) -> Result<(), NotifyError> {
///         // Deliver via your SMTP client
///         Ok(())
///     }
/// }
/// ```
///
/// ## Injecting into Axum State
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use scyph_notify::{EmailService, NoEmailService};
///
/// // In tests, swap LettreSMTPService for NoEmailService without changing handlers:
/// let svc: Arc<dyn EmailService> = Arc::new(NoEmailService);
/// let app = axum::Router::new()
///     .route("/register", axum::routing::post(register_handler))
///     .layer(axum::extract::Extension(svc));
/// ```
pub trait EmailService: Send + Sync + 'static {
    /// Sends an [`EmailMessage`] payload asynchronously.
    fn send(&self, msg: EmailMessage) -> impl Future<Output = Result<(), NotifyError>> + Send;

    #[cfg(feature = "email")]
    /// Renders a strongly-typed [`EmailTemplate`] using [`TemplateEngine`](crate::template::TemplateEngine) and sends the rendered message.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if template context serialization fails, template rendering fails, or SMTP transmission fails.
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

    /// Spawns a background Tokio task to send an email payload asynchronously without blocking the calling handler or thread.
    ///
    /// Any failure during background delivery is logged via `tracing::error!` without propagating an error to the API caller.
    fn send_background(self: Arc<Self>, msg: EmailMessage) {
        tokio::spawn(async move {
            if let Err(err) = self.send(msg).await {
                tracing::error!(error = %err, "Failed to send background email notification");
            }
        });
    }

    #[cfg(feature = "email")]
    /// Spawns a background Tokio task to render and send a templated email asynchronously without blocking the calling handler.
    ///
    /// Any failure during template compilation or delivery is logged via `tracing::error!`.
    fn send_template_background<E: EmailTemplate + Send + Sync + 'static>(
        self: Arc<Self>,
        engine: Arc<crate::template::TemplateEngine>,
        template: E,
    ) {
        tokio::spawn(async move {
            if let Err(err) = self.send_template(&engine, &template).await {
                tracing::error!(error = %err, "Failed to send background templated email notification");
            }
        });
    }
}
