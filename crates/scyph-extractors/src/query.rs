//! Axum query parameter extractors with `garde` validation.

use crate::error::ExtractorError;
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use garde::Validate;
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

/// Axum extractor for query string parameters with `garde` validation.
///
/// Automatically parses query parameters into `T` and validates domain rules.
///
/// # Examples
///
/// ```rust,ignore
/// use garde::Validate;
/// use scyph_extractors::query::ValidatedQuery;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Validate)]
/// pub struct SearchFilter {
///     #[garde(length(min = 1, max = 100))]
///     pub q: String,
/// }
///
/// async fn search_handler(
///     ValidatedQuery(filter): ValidatedQuery<SearchFilter>,
/// ) -> &'static str {
///     println!("Searching for: {}", filter.q);
///     "Search results"
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T> ValidatedQuery<T> {
    /// Consumes the wrapper, returning the inner validated query parameter struct `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for ValidatedQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync, T> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + Send + 'static,
    <T as Validate>::Context: Default,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ExtractorError::QueryParse(e.to_string()))?;

        value
            .validate_with(&Default::default())
            .map_err(|e| ExtractorError::Validation(e.to_string()))?;

        Ok(ValidatedQuery(value))
    }
}
