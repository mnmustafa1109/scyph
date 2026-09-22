//! Email notification traits, domain template abstractions, and asynchronous message delivery contracts.

use crate::NotifyError;
use std::sync::Arc;

/// Email message payload for dispatch via [`EmailService`].
///
/// Contains recipient addresses, subject header, HTML formatted body, and an optional plaintext alternative.
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
///         "welcome.html"
///     }
///
///     fn context(&self) -> serde_json::Value {
///         json!({ "name": self.user_name })
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
