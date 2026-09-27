//! Reusable query parameter structures for pagination, sorting, filtering, and searching.

use garde::Validate;
use scyph_core::PagedResponse;
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt::Display, str::FromStr};

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
///
/// Converts query string digits or raw integers into target numeric type `T`.
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

impl PaginationParams {
    /// Constructs a [`PagedResponse`] envelope from a vector of items and total count.
    ///
    /// Automatically converts `page` and `limit` to `i64` required by [`PagedResponse`].
    ///
    /// # Arguments
    ///
    /// * `data` - Page item records vector.
    /// * `total` - Total count of matching records across all pages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_extractors::query::PaginationParams;
    ///
    /// let pagination = PaginationParams { page: 1, limit: 10 };
    /// let items = vec!["alice", "bob"];
    /// let paged = pagination.into_paged_response(items, 42);
    ///
    /// assert_eq!(paged.total, 42);
    /// assert_eq!(paged.page, 1);
    /// assert_eq!(paged.per_page, 10);
    /// assert!(paged.has_next);
    /// ```
    pub fn into_paged_response<T: Serialize>(self, data: Vec<T>, total: i64) -> PagedResponse<T> {
        PagedResponse::new(data, total, self.page as i64, self.limit as i64)
    }

    /// Constructs a [`PagedResponse`] envelope referencing this pagination configuration.
    pub fn to_paged_response<T: Serialize>(&self, data: Vec<T>, total: i64) -> PagedResponse<T> {
        PagedResponse::new(data, total, self.page as i64, self.limit as i64)
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

impl<S, F> RequestParams<S, F> {
    /// Convenience helper to create a [`PagedResponse`] consuming this request's pagination settings.
    pub fn into_paged_response<T: Serialize>(self, data: Vec<T>, total: i64) -> PagedResponse<T> {
        self.pagination.into_paged_response(data, total)
    }

    /// Convenience helper to create a [`PagedResponse`] referencing this request's pagination settings.
    pub fn to_paged_response<T: Serialize>(&self, data: Vec<T>, total: i64) -> PagedResponse<T> {
        self.pagination.to_paged_response(data, total)
    }
}
