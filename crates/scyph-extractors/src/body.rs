//! Axum request body extractors with automatic sanitization and `garde` validation.

use crate::error::ExtractorError;
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use garde::Validate;
use sanitizer::Sanitizer;
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

/// Axum extractor for JSON request bodies with automatic input sanitization and `garde` validation.
///
/// 1. Deserializes raw JSON bytes into `T`.
/// 2. Applies string sanitization via [`Sanitizer::sanitize`].
/// 3. Validates domain constraints via [`Validate::validate_with`].
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

        value
            .validate_with(&Default::default())
            .map_err(|e| ExtractorError::Validation(e.to_string()))?;

        Ok(ValidatedJson(value))
    }
}

/// Axum extractor for JSON request bodies that performs string sanitization without `garde` validation.
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
