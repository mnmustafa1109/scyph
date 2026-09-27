//! Axum query parameter extractor with `garde` validation.

use crate::error::ExtractorError;
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use garde::Validate;
use scyph_core::error::{AppError, ErrorDetails};
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

/// Axum extractor for query string parameters with `garde` validation.
///
/// Automatically parses query parameters into `T` and validates domain rules.
///
/// Converts validation failures into structured [`AppError::ValidationError`] containing field-level error details.
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

        if let Err(report) = value.validate_with(&Default::default()) {
            let details: Vec<ErrorDetails> = report
                .iter()
                .map(|(path, error)| {
                    let field = path.to_string();
                    ErrorDetails {
                        code: "INVALID_QUERY_PARAM".to_string(),
                        message: error.to_string(),
                        field: if field.is_empty() { None } else { Some(field) },
                    }
                })
                .collect();

            return Err(AppError::ValidationError {
                message: "Query parameter validation failed".to_string(),
                details,
            });
        }

        Ok(ValidatedQuery(value))
    }
}
