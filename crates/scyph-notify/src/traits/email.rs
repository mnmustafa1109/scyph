//! Email notification traits and data models.

use crate::NotifyError;
use std::sync::Arc;

/// Email message payload for dispatch via [`EmailService`].
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// Recipient email addresses.
    pub to: Vec<String>,
    /// Subject line of the email.
    pub subject: String,
    /// HTML formatted body content.
    pub html: String,
    /// Optional plaintext body fallback.
    pub text: Option<String>,
}

/// Contract for strongly-typed domain email templates.
pub trait EmailTemplate {
    /// Recipient email addresses.
    fn to(&self) -> Vec<String>;

    /// Subject line of the email.
    fn subject(&self) -> String;

    /// Name of the Tera template file (e.g. `"new_message.html"`).
    fn template_name(&self) -> &str;

    /// Template context variables serialized as JSON [`serde_json::Value`].
    fn context(&self) -> serde_json::Value;

    /// Optional plaintext body fallback.
    fn text(&self) -> Option<String> {
        None
    }
}

/// Abstract contract for asynchronous email delivery services.
pub trait EmailService: Send + Sync + 'static {
    /// Sends an [`EmailMessage`] asynchronously.
    fn send(&self, msg: EmailMessage) -> impl Future<Output = Result<(), NotifyError>> + Send;

    #[cfg(feature = "email")]
    /// Renders a strongly-typed [`EmailTemplate`] using [`TemplateEngine`](crate::template::TemplateEngine) and sends the email asynchronously.
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

    /// Spawns a background Tokio task to send the email asynchronously without blocking the caller.
    fn send_background(self: Arc<Self>, msg: EmailMessage) {
        tokio::spawn(async move {
            if let Err(err) = self.send(msg).await {
                tracing::error!(error = %err, "Failed to send background email notification");
            }
        });
    }

    #[cfg(feature = "email")]
    /// Spawns a background Tokio task to render and send a templated email asynchronously without blocking the caller.
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
