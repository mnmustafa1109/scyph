//! Tera template rendering engine wrapper.

use scyph_core::AppError;
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
    /// Returns [`AppError::Internal`] if loading or parsing templates fails.
    pub fn from_glob(glob: &str) -> Result<Self, AppError> {
        let mut tera = Tera::default();
        tera.load_from_glob(glob)
            .map_err(|e| AppError::internal_from(e, "load email templates"))?;
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
    /// Returns [`AppError::Internal`] if template rendering fails.
    pub fn render(&self, template: &str, ctx: &tera::Context) -> Result<String, AppError> {
        self.tera
            .render(template, ctx)
            .map_err(|e| AppError::internal_from(e, format!("render template '{template}'")))
    }
}
