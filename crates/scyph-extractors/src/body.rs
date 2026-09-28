//! Axum request body extractors with automatic sanitization and `garde` validation.
//!
//! ## Sanitization Pipeline
//!
//! Both extractors in this module use the `sanitizer` crate's `Sanitizer` derive macro to
//! apply string transformations (trimming, case conversion, etc.) **before** validation runs.
//! This prevents spurious validation failures from leading/trailing whitespace and ensures
//! consistent data storage.
//!
//! ## `garde::Validate` Context Type
//!
//! Both [`ValidatedJson`] and [`ValidatedPath`](crate::path::ValidatedPath) call
//! `validate_with(&Default::default())`. The context type is inferred from the
//! `<T as Validate>::Context` associated type, which defaults to `()` for most structs.
//!
//! If your validation rules require a custom context (e.g. database-level uniqueness checks),
//! you need to provide it before calling the extractor — in that case, implement a custom
//! `FromRequest` that builds the context and calls `validate_with` manually, rather than
//! using `ValidatedJson` directly.

use crate::error::ExtractorError;
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use garde::Validate;
use sanitizer::Sanitizer;
use scyph_core::error::{AppError, ErrorDetails};
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

/// Axum extractor for JSON request bodies with automatic input sanitization and `garde` validation.
///
/// Execution Pipeline:
/// 1. Deserializes raw JSON bytes into `T`.
/// 2. Applies string sanitization via [`Sanitizer::sanitize`] (e.g. whitespace trimming, case conversions).
/// 3. Validates domain constraints via [`Validate::validate_with`].
///
/// Converts validation failures into structured [`AppError::ValidationError`] containing field-level error details.
///
/// # Examples
///
/// ```rust,ignore
/// use garde::Validate;
/// use sanitizer::Sanitizer;
/// use scyph_extractors::body::ValidatedJson;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Validate, Sanitizer)]
/// pub struct CreateUser {
///     #[sanitizer(trim)]
///     #[garde(length(min = 1))]
///     pub name: String,
/// }
///
/// async fn create_user_handler(
///     ValidatedJson(payload): ValidatedJson<CreateUser>,
/// ) -> &'static str {
///     println!("Creating user: {}", payload.name);
///     "User created"
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    /// Consumes the wrapper, returning the inner sanitized and validated value `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Sanitizer + 'static,
    <T as Validate>::Context: Default,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(mut value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ExtractorError::JsonParse(e.to_string()))?;

        value.sanitize();

        if let Err(report) = value.validate_with(&Default::default()) {
            let details: Vec<ErrorDetails> = report
                .iter()
                .map(|(path, error)| {
                    let field = path.to_string();
                    ErrorDetails {
                        code: "VALIDATION_ERROR".to_string(),
                        message: error.to_string(),
                        field: if field.is_empty() { None } else { Some(field) },
                    }
                })
                .collect();

            return Err(AppError::ValidationError {
                message: "JSON body payload validation failed".to_string(),
                details,
            });
        }

        Ok(ValidatedJson(value))
    }
}

/// Axum extractor for JSON request bodies that performs string sanitization without `garde` validation.
///
/// Use this when you want automatic sanitization (trimming, case normalization, etc.) but do not
/// need `garde` validation rules — for example, when validation is handled separately or when
/// the struct does not derive `garde::Validate`.
///
/// ## When to Use `SanitizedJson` vs. `ValidatedJson`
///
/// | | `SanitizedJson<T>` | `ValidatedJson<T>` |
/// |---|---|---|
/// | String sanitization | ✅ | ✅ |
/// | `garde` validation | ❌ | ✅ |
/// | `T` constraint | `Sanitizer` only | `Sanitizer + Validate` |
///
/// # Examples
///
/// ```rust,ignore
/// use sanitizer::Sanitizer;
/// use scyph_extractors::body::SanitizedJson;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Sanitizer)]
/// pub struct SearchQuery {
///     /// Trimmed and lowercased automatically before handler runs
///     #[sanitizer(trim, lower_case)]
///     pub q: String,
///     /// Whitespace trimmed from both ends
///     #[sanitizer(trim)]
///     pub category: Option<String>,
/// }
///
/// async fn search_handler(
///     SanitizedJson(query): SanitizedJson<SearchQuery>,
/// ) -> String {
///     // query.q is already trimmed and lowercased
///     format!("Searching for: {}", query.q)
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SanitizedJson<T>(pub T);

impl<T> SanitizedJson<T> {
    /// Consumes the wrapper, returning the inner sanitized value `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for SanitizedJson<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for SanitizedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync, T> FromRequest<S> for SanitizedJson<T>
where
    T: DeserializeOwned + Sanitizer + 'static,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(mut value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ExtractorError::JsonParse(e.to_string()))?;

        value.sanitize();

        Ok(SanitizedJson(value))
    }
}
