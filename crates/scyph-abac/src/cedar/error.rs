//! Cedar policy engine error types.
//!
//! Defines [`CedarError`] — failures from AWS Cedar policy parsing, schema validation,
//! request construction, and authorization evaluation.
//!
//! ## Error Hierarchy
//!
//! ```text
//! CedarError
//! ├── PolicyParse   — Cedar policy DSL syntax error
//! ├── SchemaParse   — Cedar schema syntax error
//! ├── RequestBuild  — Entity UID resolution or request construction failure
//! ├── Evaluation    — Cedar evaluator runtime error
//! └── AccessDenied  — Policy returned Decision::Deny (authorization failure)
//! ```
//!
//! `AccessDenied` converts to HTTP 403 Forbidden. All other variants convert to HTTP 500
//! Internal Server Error — they indicate a server-side configuration or programming error
//! rather than a client access denial.

use scyph_core::AppError;
use thiserror::Error;

/// Errors encountered during Cedar policy parsing, schema validation, or request evaluation.
#[derive(Debug, Error)]
pub enum CedarError {
    /// Failed to parse Cedar policy source text.
    #[error("Policy parse error: {0}")]
    PolicyParse(String),

    /// Failed to parse Cedar schema text.
    #[error("Schema parse error: {0}")]
    SchemaParse(String),

    /// Failed to construct Cedar request or entity container.
    #[error("Request build error: {0}")]
    RequestBuild(String),

    /// Evaluation error encountered during authorization.
    #[error("Evaluation error: {0}")]
    Evaluation(String),

    /// Authorization evaluation resulted in `Decision::Deny`.
    #[error("Access denied")]
    AccessDenied,
}

impl From<CedarError> for AppError {
    fn from(e: CedarError) -> Self {
        match e {
            CedarError::AccessDenied => AppError::Forbidden("Not Permitted".into()),
            other => AppError::internal_from(other, "Cedar authorization error"),
        }
    }
}
