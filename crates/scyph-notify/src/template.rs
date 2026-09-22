//! Tera template rendering engine wrapper.

use crate::NotifyError;
use crate::traits::{EmailMessage, EmailTemplate};
use tera::Tera;

/// Tera template engine wrapper for parsing and rendering HTML/text templates.
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    /// Constructs a [`TemplateEngine`] by loading templates matching a file glob pattern.
    ///
    /// # Arguments
    ///
    /// * `glob` - File glob pattern (e.g. `"templates/**/*.html"`).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if loading or parsing templates fails.
    pub fn from_glob(glob: &str) -> Result<Self, NotifyError> {
        let mut tera = Tera::default();
        tera.load_from_glob(glob)?;
        Ok(Self { tera })
    }

    /// Renders a compiled template with the provided [`tera::Context`].
    ///
    /// # Arguments
    ///
    /// * `template` - Name of the template to render.
    /// * `ctx` - Template context containing template variables.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if template rendering fails.
    pub fn render(&self, template: &str, ctx: &tera::Context) -> Result<String, NotifyError> {
        let rendered = self.tera.render(template, ctx)?;
        Ok(rendered)
    }

    /// Renders a strongly-typed [`EmailTemplate`] into an [`EmailMessage`].
    ///
    /// If `template.text()` is `None` and the HTML template name ends with `.html` (e.g. `"welcome.html"`),
    /// automatically attempts to render a matching `.txt` template (e.g. `"welcome.txt"`) for plaintext body fallback.
    ///
    /// # Arguments
    ///
    /// * `template` - Strongly-typed email template implementing [`EmailTemplate`].
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if serializing context or rendering fails.
    pub fn render_email<E: EmailTemplate>(
        &self,
        template: &E,
    ) -> Result<EmailMessage, NotifyError> {
        let ctx = tera::Context::from_serialize(&template.context())?;
        let html_name = template.template_name();
        let html = self.render(html_name, &ctx)?;

        let text = if let Some(t) = template.text() {
            Some(t)
        } else if html_name.ends_with(".html") {
            let txt_name = html_name.replace(".html", ".txt");
            self.render(&txt_name, &ctx).ok()
        } else {
            None
        };

        Ok(EmailMessage {
            to: template.to(),
            subject: template.subject(),
            html,
            text,
        })
    }
}
