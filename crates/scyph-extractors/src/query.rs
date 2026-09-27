//! Axum query parameter extractors with `garde` validation and standard pagination/sorting models.

use crate::error::ExtractorError;
use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use garde::Validate;
use scyph_core::error::{AppError, ErrorDetails};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// Sorting direction enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending sort order (`"asc"`).
    #[default]
    Asc,
    /// Descending sort order (`"desc"`).
    Desc,
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asc => write!(f, "asc"),
            Self::Desc => write!(f, "desc"),
        }
    }
}

/// Helper deserializer to parse numbers from either raw JSON integers or query string numbers (`"10"` -> `10`).
pub fn deserialize_number_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr + Deserialize<'de>,
    T::Err: Display,
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber<T> {
        Number(T),
        String(String),
    }

    match StringOrNumber::<T>::deserialize(deserializer)? {
        StringOrNumber::Number(n) => Ok(n),
        StringOrNumber::String(s) => s.parse::<T>().map_err(serde::de::Error::custom),
    }
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    10
}

/// Standardized offset pagination query parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct PaginationParams {
    /// 1-indexed page number (default: `1`).
    #[serde(
        default = "default_page",
        deserialize_with = "deserialize_number_from_string"
    )]
    #[garde(range(min = 1))]
    pub page: u32,

    /// Number of items per page (default: `10`, max: `100`).
    #[serde(
        default = "default_limit",
        deserialize_with = "deserialize_number_from_string"
    )]
    #[garde(range(min = 1, max = 100))]
    pub limit: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            limit: default_limit(),
        }
    }
}

/// Generic sorting query parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct SortParams<S> {
    /// Target sort field identifier.
    #[garde(skip)]
    pub sort_by: Option<S>,

    /// Sort direction (`Asc` or `Desc`).
    #[garde(skip)]
    pub sort_order: Option<SortOrder>,
}

impl<S> Default for SortParams<S> {
    fn default() -> Self {
        Self {
            sort_by: None,
            sort_order: None,
        }
    }
}

/// Generic filtering query parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct FilterParams<F> {
    /// Target filter column identifier.
    #[garde(skip)]
    pub filter_by: Option<F>,

    /// Value to filter on.
    #[garde(skip)]
    pub filter_value: Option<String>,
}

impl<F> Default for FilterParams<F> {
    fn default() -> Self {
        Self {
            filter_by: None,
            filter_value: None,
        }
    }
}

/// Search query string parameter (`?q=term`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, Default)]
pub struct SearchParams {
    /// Search term query string.
    #[garde(skip)]
    pub q: Option<String>,
}

/// Composite request parameter wrapper for combined pagination, sorting, filtering, and searching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, Default)]
pub struct RequestParams<S, F> {
    /// Flattened pagination parameter fields (`page`, `limit`).
    #[serde(flatten)]
    #[garde(dive)]
    pub pagination: PaginationParams,

    /// Flattened sort parameter fields (`sort_by`, `sort_order`).
    #[serde(flatten)]
    #[garde(dive)]
    pub sort: SortParams<S>,

    /// Flattened filter parameter fields (`filter_by`, `filter_value`).
    #[serde(flatten)]
    #[garde(dive)]
    pub filter: FilterParams<F>,

    /// Flattened search parameter fields (`q`).
    #[serde(flatten)]
    #[garde(dive)]
    pub search: SearchParams,
}

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
