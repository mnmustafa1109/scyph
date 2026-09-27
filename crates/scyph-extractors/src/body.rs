// crates/scyph-extractors/src/body.rs
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use garde::Validate;
use sanitizer::Sanitizer;
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use std::ops::Deref;

pub struct ValidatedJson<T>(pub T);

impl<T> Deref for ValidatedJson<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<S: Send + Sync, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Sanitizer + 'static,
    <T as Validate>::Context: Default,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, AppError> {
        let Json(mut value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {e}")))?;
        value.sanitize();
        value
            .validate_with(&Default::default())
            .map_err(|e| AppError::UnprocessableEntity(e.to_string()))?;
        Ok(ValidatedJson(value))
    }
}
