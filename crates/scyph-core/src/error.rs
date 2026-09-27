//! Standardized HTTP error handling for Scyph backends.
//!
//! This module provides [`AppError`], the canonical error envelope used across
//! Scyph services. It follows the [RFC 7807 Problem Details](https://tools.ietf.org/html/rfc7807)
//! format for HTTP APIs.
//!
//! # Key Features
//! - **RFC 7807 Compliance**: Serializes responses with `type`, `title`, `status`, and `detail`.
//! - **Automatic Response Mapping**: Implements [`IntoResponse`](axum::response::IntoResponse) to easily return errors from Axum handlers.
//! - **Security First**: Internal server errors mask sensitive implementation details from clients while logging full tracebacks.
//!
//! # Examples
//!
//! ```rust
//! use scyph_core::AppError;
//! use axum::http::StatusCode;
//!
//! fn find_user(id: u64) -> Result<String, AppError> {
//!     if id == 0 {
//!         Err(AppError::NotFound("User not found".into()))
//!     } else {
//!         Ok("Alice".into())
//!     }
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// Field-level detail for request payload validation or processing errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Machine-readable error code string (e.g., `"INVALID_EMAIL"`, `"REQUIRED"`).
    pub code: String,
    /// Human-readable description of the error.
    pub message: String,
    /// Optional field name or payload property path (e.g., `"email"`, `"pagination.limit"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// The canonical error type for all Scyph-based HTTP backends.
///
/// `AppError` maps domain and runtime errors into standardized HTTP status codes
/// and RFC 7807 Problem Details JSON responses when returned from Axum handlers.
///
/// Internal errors ([`AppError::Internal`]) log the underlying source error using `tracing::error!`
/// while returning an opaque message to clients, preventing internal leaks.
#[derive(Debug, Error)]
pub enum AppError {
    /// 400 Bad Request — Malformed or invalid input that the client can fix.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// 401 Unauthorized — Missing, invalid, or expired authentication credentials.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// 403 Forbidden — Authenticated entity lacks permission to access the resource.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// 404 Not Found — Requested resource could not be found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// 409 Conflict — Resource state conflict (e.g., duplicate email address on registration).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// 422 Unprocessable Entity — Request syntax is correct but validation failed.
    #[error("Unprocessable: {0}")]
    UnprocessableEntity(String),

    /// 422 Unprocessable Entity — Structured request validation failure with field-level details.
    #[error("Validation failed: {message}")]
    ValidationError {
        /// Summary validation failure message.
        message: String,
        /// List of field-level validation errors.
        details: Vec<ErrorDetails>,
    },

    /// 402 Payment Required — Access requires payment or active subscription.
    #[error("Payment required: {0}")]
    PaymentRequired(String),

    /// 429 Too Many Requests — Client has exceeded rate limit thresholds.
    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    /// 503 Service Unavailable — Server is temporarily overloaded or under maintenance.
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// 500 Internal Server Error — Unexpected internal failure.
    ///
    /// The full error context is logged via `tracing::error!`, but the client receives an
    /// opaque generic detail message `"An unexpected error occurred."` to preserve security.
    #[error("Internal server error")]
    Internal {
        /// The underlying source error causing the failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        /// Descriptive contextual summary of where/why the error occurred.
        context: String,
    },

    /// Custom error variant for domain-specific HTTP responses.
    #[error("{message}")]
    Custom {
        /// HTTP status code for the response.
        status: StatusCode,
        /// Machine-readable error code string (used in RFC 7807 `title`).
        code: String,
        /// Client-facing detail message.
        message: String,
    },
}

impl AppError {
    /// Wraps any standard error (`std::error::Error`) into an internal server error variant ([`AppError::Internal`]).
    ///
    /// The underlying error will be logged internally via `tracing`, while the response detail sent
    /// to the client is masked for security.
    ///
    /// # Arguments
    ///
    /// * `err` - The underlying source error.
    /// * `ctx` - A human-readable context description explaining where the error occurred.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::AppError;
    ///
    /// let parse_result: Result<i32, _> = "invalid".parse();
    /// let app_err = parse_result.map_err(|e| AppError::internal_from(e, "Failed to parse integer"));
    /// assert_eq!(app_err.unwrap_err().status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    /// ```
    pub fn internal_from<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        ctx: impl Into<String>,
    ) -> Self {
        Self::Internal {
            source: Box::new(err),
            context: ctx.into(),
        }
    }

    /// Creates an internal server error ([`AppError::Internal`]) from a plain string message context
    /// without a separate underlying source error.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A message explaining the context of the internal failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::AppError;
    ///
    /// let err = AppError::internal("Database connection pool exhausted");
    /// assert_eq!(err.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    /// ```
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

    /// Returns the HTTP status code associated with this error variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::AppError;
    /// use axum::http::StatusCode;
    ///
    /// let err = AppError::NotFound("Item missing".into());
    /// assert_eq!(err.status(), StatusCode::NOT_FOUND);
    /// ```
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UnprocessableEntity(_) | Self::ValidationError { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Custom { status, .. } => *status,
        }
    }

    /// Returns a machine-readable string error code identifier (e.g., `"NOT_FOUND"`, `"UNAUTHORIZED"`).
    ///
    /// This string populates the `title` field in the RFC 7807 JSON error response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::AppError;
    ///
    /// let err = AppError::Unauthorized("Invalid token".into());
    /// assert_eq!(err.code(), "UNAUTHORIZED");
    /// ```
    pub fn code(&self) -> &str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
            Self::PaymentRequired(_) => "PAYMENT_REQUIRED",
            Self::TooManyRequests(_) => "TOO_MANY_REQUESTS",
            Self::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
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
        let mut body = json!({
            "type":   format!("https://httpstatuses.io/{}", status.as_u16()),
            "title":  self.code(),
            "status": status.as_u16(),
            "detail": match &self {
                Self::Internal { .. } => "An unexpected error occurred.".to_string(),
                Self::ValidationError { message, .. } => message.clone(),
                other => other.to_string(),
            },
        });

        if let Self::ValidationError { details, .. } = &self {
            body.as_object_mut()
                .unwrap()
                .insert("details".to_string(), json!(details));
        }

        (status, axum::Json(body)).into_response()
    }
}
