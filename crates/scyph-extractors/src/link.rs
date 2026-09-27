//! Header-aware canonical link generator for Axum requests.

use axum::{extract::FromRequestParts, http::request::Parts};
use scyph_core::error::AppError;
use url::Url;

/// Canonical URL link generator extracted from incoming request headers (`Host`, `X-Forwarded-Proto`).
#[derive(Debug, Clone)]
pub struct LinkGenerator {
    base_url: Url,
}

impl LinkGenerator {
    /// Returns the parsed base URL instance.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Creates a new [`LinkBuilder`] pre-configured with the request's canonical base URL.
    pub fn build(&self) -> LinkBuilder {
        LinkBuilder {
            url: self.base_url.clone(),
        }
    }
}

/// Builder for fluently constructing absolute URLs and endpoint links.
#[derive(Debug, Clone)]
pub struct LinkBuilder {
    url: Url,
}

impl LinkBuilder {
    /// Appends a URL path segment.
    pub fn path(mut self, path: &str) -> Self {
        if let Ok(mut segments) = self.url.path_segments_mut() {
            segments.push(path.trim_start_matches('/'));
        }
        self
    }

    /// Appends an API version string path segment (e.g. `"v1"`).
    pub fn version(self, v: &str) -> Self {
        self.path(v)
    }

    /// Extends query parameters.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(key, value);
        self
    }

    /// Consumes the builder and returns the completed URL string.
    pub fn finish(self) -> String {
        self.url.to_string()
    }
}

impl<S> FromRequestParts<S> for LinkGenerator
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let host = parts
            .headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::BadRequest("Missing Host header".to_string()))?;

        let scheme = parts
            .headers
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("http");

        let base_url_str = format!("{}://{}", scheme, host);
        let base_url = Url::parse(&base_url_str)
            .map_err(|e| AppError::BadRequest(format!("Failed to parse base URL: {e}")))?;

        Ok(LinkGenerator { base_url })
    }
}
