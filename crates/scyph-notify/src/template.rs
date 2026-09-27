//! Tera template rendering engine wrapper.
//!
//! Provides [`TemplateEngine`] for parsing, compiling, and rendering HTML and plaintext templates
//! from a local directory or custom glob pattern.

use std::env;

use crate::NotifyError;
use crate::traits::{EmailMessage, EmailTemplate};
use tera::Tera;

/// Tera template engine wrapper for compiling and rendering HTML/text email and notification templates.
///
/// Encapsulates a compiled [`Tera`] instance. Supports loading templates from environment variables (`TEMPLATES_DIR`),
/// directory paths (`from_dir`), or file glob patterns (`from_glob`).
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    /// Constructs a [`TemplateEngine`] by reading `TEMPLATES_DIR` (defaulting to `"templates/**/*"`).
    ///
    /// Loads and compiles all Tera templates located in the project's root `templates/` directory.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if syntax error occurs during template compilation.
    pub fn from_env() -> Result<Self, NotifyError> {
        let glob = env::var("TEMPLATES_DIR").unwrap_or_else(|_| "templates/**/*".to_string());
        Self::from_glob(&glob)
    }

    /// Constructs a [`TemplateEngine`] by loading all template files recursively from a root folder path.
    ///
    /// # Arguments
    ///
    /// * `dir` - Base templates directory (e.g. `"templates"` or `"src/templates"`).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if loading or parsing templates fails.
    pub fn from_dir(dir: &str) -> Result<Self, NotifyError> {
        let pattern = format!("{}/**/*", dir.trim_end_matches('/'));
        Self::from_glob(&pattern)
    }

    /// Constructs a [`TemplateEngine`] by compiling templates matching a custom file glob pattern.
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

    /// Renders a compiled template by name using the provided [`tera::Context`].
    ///
    /// # Arguments
    ///
    /// * `template` - Name of the registered template file (e.g., `"welcome.html"`).
    /// * `ctx` - Template context variables.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if the template is not found or rendering fails.
    pub fn render(&self, template: &str, ctx: &tera::Context) -> Result<String, NotifyError> {
        let rendered = self.tera.render(template, ctx)?;
        Ok(rendered)
    }

    /// Renders a strongly-typed [`EmailTemplate`] into a complete [`EmailMessage`] payload.
    ///
    /// ### Plaintext Fallback Resolution
    /// 1. If `template.text()` returns `Some(...)`, uses that explicit plaintext string.
    /// 2. If `template.text()` is `None` and the template name ends with `.html` (e.g. `"welcome.html"`),
    ///    automatically attempts to render a matching `.txt` file (e.g. `"welcome.txt"`) using the same context.
    /// 3. If no matching `.txt` file exists, plaintext body is set to `None`.
    ///
    /// # Arguments
    ///
    /// * `template` - Strongly-typed email template struct.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Template`] if context serialization or HTML template rendering fails.
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
