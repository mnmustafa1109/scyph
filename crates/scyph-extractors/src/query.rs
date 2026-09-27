// crates/scyph-extractors/src/path.rs
use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use std::ops::Deref;

pub struct TypedPath<T>(pub T);

impl<T> Deref for TypedPath<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<S: Send + Sync, T: DeserializeOwned + Send + 'static> FromRequestParts<S> for TypedPath<T> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid path parameter: {e}")))?;
        Ok(TypedPath(value))
    }
}
