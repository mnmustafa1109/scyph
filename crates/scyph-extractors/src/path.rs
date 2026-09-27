// crates/scyph-extractors/src/query.rs
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use garde::Validate;
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use std::ops::Deref;

pub struct ValidatedQuery<T>(pub T);

impl<T> Deref for ValidatedQuery<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<S: Send + Sync, T> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + Send + 'static,
    <T as Validate>::Context: Default,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid query parameters: {e}")))?;
        value
            .validate_with(&Default::default())
            .map_err(|e| AppError::UnprocessableEntity(e.to_string()))?;
        Ok(ValidatedQuery(value))
    }
}
