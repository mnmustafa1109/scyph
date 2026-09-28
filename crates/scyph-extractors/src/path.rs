//! Axum URL path parameter extractors.
//!
//! This module provides two extractors for Axum path parameters:
//!
//! - [`TypedPath<T>`]: Thin wrapper around `axum::extract::Path<T>` that converts parse errors
//!   into [`AppError`] automatically. Use for simple scalar types (`Uuid`, `i64`, etc.).
//!
//! - [`ValidatedPath<T>`]: Extends `TypedPath` with `garde` validation. Use when the path
//!   parameter is a struct that needs constraint validation (e.g. range checks on an integer ID).
//!
//! ## Context Type Note
//!
//! [`ValidatedPath`] calls `validate_with(&Default::default())`. The `garde::Validate::Context`
//! associated type must implement `Default`. For most structs this is `()`, which satisfies
//! the bound automatically.

use crate::error::ExtractorError;
use axum::{
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use garde::Validate;
use scyph_core::AppError;
use serde::de::DeserializeOwned;
use std::ops::{Deref, DerefMut};

/// Axum extractor for strongly-typed URL path parameters.
///
/// # Examples
///
/// ```rust,ignore
/// use scyph_extractors::path::TypedPath;
/// use uuid::Uuid;
///
/// async fn user_detail_handler(
///     TypedPath(user_id): TypedPath<Uuid>,
/// ) -> String {
///     format!("User ID is {}", user_id)
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypedPath<T>(pub T);

impl<T> TypedPath<T> {
    /// Consumes the wrapper, returning the inner path parameter `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for TypedPath<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for TypedPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync, T: DeserializeOwned + Send + 'static> FromRequestParts<S> for TypedPath<T> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ExtractorError::PathParse(e.to_string()))?;

        Ok(TypedPath(value))
    }
}

/// Axum extractor for URL path parameters with `garde` validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidatedPath<T>(pub T);

impl<T> ValidatedPath<T> {
    /// Consumes the wrapper, returning the inner validated path parameter struct `T`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedPath<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for ValidatedPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync, T> FromRequestParts<S> for ValidatedPath<T>
where
    T: DeserializeOwned + Validate + Send + 'static,
    <T as Validate>::Context: Default,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ExtractorError::PathParse(e.to_string()))?;

        value
            .validate_with(&Default::default())
            .map_err(|e| ExtractorError::Validation(e.to_string()))?;

        Ok(ValidatedPath(value))
    }
}
