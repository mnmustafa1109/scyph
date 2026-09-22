//! Tera template rendering engine wrapper.

use crate::NotifyError;
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
}
