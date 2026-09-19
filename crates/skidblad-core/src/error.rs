// crates/skidblad-core/src/error.rs
//
// Shape follows RFC 7807 Problem Details for HTTP APIs.
// Internal errors are logged via tracing; clients receive an opaque message.
// Return AppError directly from Axum handlers — it implements IntoResponse.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// The canonical error type for all skidblad-based backends.
///
/// Return it directly from Axum handlers — it implements `IntoResponse`.
/// Every variant maps to a specific HTTP status code and a `code` string
/// that goes in the RFC 7807 `title` field.
#[derive(Debug, Error)]
pub enum AppError {
    /// 400 Bad Request — malformed input the client can fix
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// 401 Unauthorized — missing or invalid credentials
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// 403 Forbidden — authenticated but not permitted
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// 404 Not Found
    #[error("Not found: {0}")]
    NotFound(String),

    /// 409 Conflict — e.g. duplicate email on registration
    #[error("Conflict: {0}")]
    Conflict(String),

    /// 422 Unprocessable Entity — validation failed
    #[error("Unprocessable: {0}")]
    UnprocessableEntity(String),

    /// 402 Payment Required — subscription gate
    #[error("Payment required: {0}")]
    PaymentRequired(String),

    /// 500 Internal Server Error — the real error is logged; clients get an
    /// opaque message so implementation details don't leak.
    #[error("Internal server error")]
    Internal {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        context: String,
    },

    /// For domain-specific variants your project needs that don't fit above.
    #[error("{message}")]
    Custom {
        status: StatusCode,
        code: String,
        message: String,
    },
}

impl AppError {
    /// Wrap any `std::error::Error` as an internal error.
    /// The original error is logged; the client sees only "An unexpected error occurred."
    pub fn internal_from<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        ctx: impl Into<String>,
    ) -> Self {
        Self::Internal {
            source: Box::new(err),
            context: ctx.into(),
        }
    }

    /// Create an internal error from a plain string context (no source error).
    pub fn internal(ctx: impl Into<String>) -> Self {
        let ctx = ctx.into();
        struct StringError(String);
        impl std::fmt::Debug for StringError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl std::fmt::Display for StringError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl std::error::Error for StringError {}
        Self::Internal {
            source: Box::new(StringError(ctx.clone())),
            context: ctx,
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Custom { status, .. } => *status,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            Self::PaymentRequired(_) => "PAYMENT_REQUIRED",
            Self::Internal { .. } => "INTERNAL_SERVER_ERROR",
            Self::Custom { code, .. } => code.as_str(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::Internal {
            ref source,
            ref context,
        } = self
        {
            error!(
                error = %source,
                context = %context,
                "Internal server error"
            );
        }

        let status = self.status();
        let body = json!({
            "type":   format!("https://httpstatuses.io/{}", status.as_u16()),
            "title":  self.code(),
            "status": status.as_u16(),
            "detail": match &self {
                Self::Internal { .. } => "An unexpected error occurred.".to_string(),
                other => other.to_string(),
            },
        });

        (status, axum::Json(body)).into_response()
    }
}
