//! Header-aware canonical link generator for Axum requests.

use std::sync::OnceLock;

use axum::{extract::FromRequestParts, http::request::Parts};
use scyph_core::AppError;
use url::Url;

/// Process-wide cache of startup env vars (`APP_BASE_URL`, `ALLOWED_HOSTS`) so requests do not
/// touch `std::env` per invocation. Hot-reloading env vars is not supported.
struct LinkEnv {
    base_url: Option<String>,
    allowed_hosts: Option<Vec<String>>,
}

fn link_env() -> &'static LinkEnv {
    static LINK_ENV: OnceLock<LinkEnv> = OnceLock::new();
    LINK_ENV.get_or_init(|| LinkEnv {
        base_url: std::env::var("APP_BASE_URL").ok(),
        allowed_hosts: std::env::var("ALLOWED_HOSTS")
            .ok()
            .map(|raw| raw.split(',').map(|h| h.trim().to_string()).collect()),
    })
}

/// Canonical URL link generator extracted from incoming request headers (`X-Forwarded-Host`, `Host`, `X-Forwarded-Proto`).
///
/// Automatically handles proxy headers behind reverse proxies (Cloudflare, NGINX, AWS ALB).
///
/// ### Security Guarantees & Configuration
///
/// 1. **Canonical Override (`APP_BASE_URL`)**: If the `APP_BASE_URL` environment variable is defined
///    (e.g., `https://api.example.com`), it takes absolute precedence as the trusted base URL,
///    completely bypassing request headers and preventing Host header poisoning attacks.
/// 2. **Header Sanitization**: Incoming `Host` and `X-Forwarded-Host` values are sanitized to reject
///    any characters used in CRLF injection, path traversal, or authority smuggling (`/`, `\`, `\r`, `\n`, space, `@`).
/// 3. **Allowed Hosts Whitelist (`ALLOWED_HOSTS`)**: If configured as a comma-separated list
///    (e.g., `api.example.com,example.com,localhost:8080`), incoming host headers are strictly validated
///    against the whitelist before acceptance.
#[derive(Debug, Clone)]
pub struct LinkGenerator {
    base_url: Url,
}

impl LinkGenerator {
    /// Constructs a new [`LinkGenerator`] with an explicit base URL.
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }

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
        // 1. If canonical APP_BASE_URL is configured, use it as trusted base URL
        if let Some(app_base) = &link_env().base_url {
            let base_url = Url::parse(app_base).map_err(|e| {
                AppError::internal(format!("Invalid APP_BASE_URL configuration: {e}"))
            })?;
            return Ok(LinkGenerator { base_url });
        }

        // 2. Extract host from headers
        let host = parts
            .headers
            .get("x-forwarded-host")
            .or_else(|| parts.headers.get("host"))
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(',').next())
            .map(|h| h.trim())
            .ok_or_else(|| AppError::BadRequest("Missing Host header".to_string()))?;

        // Basic sanity check to prevent CRLF or header splitting in host
        if host.contains(['/', '\\', '\r', '\n', ' ', '@']) {
            return Err(AppError::BadRequest("Malformed Host header".to_string()));
        }

        // 3. ALLOWED_HOSTS must be configured when falling back to a request
        //    header; trusting an arbitrary Host header enables host-header
        //    injection. Require explicit configuration in that case.
        match &link_env().allowed_hosts {
            Some(allowed_hosts) => {
                let host_without_port = host.split(':').next().unwrap_or(host);
                let is_allowed = allowed_hosts.iter().any(|a| {
                    a.eq_ignore_ascii_case(host) || a.eq_ignore_ascii_case(host_without_port)
                });
                if !is_allowed {
                    return Err(AppError::BadRequest(
                        "Untrusted Host header rejected".to_string(),
                    ));
                }
            }
            None => {
                return Err(AppError::BadRequest(
                    "ALLOWED_HOSTS or APP_BASE_URL must be configured to build links from a request host".to_string(),
                ));
            }
        }

        let scheme = parts
            .headers
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(',').next())
            .map(|h| h.trim())
            .unwrap_or("http");

        let valid_scheme = match scheme {
            "https" => "https",
            _ => "http",
        };

        let base_url_str = format!("{}://{}", valid_scheme, host);
        let base_url = Url::parse(&base_url_str)
            .map_err(|e| AppError::BadRequest(format!("Failed to parse base URL: {e}")))?;

        Ok(LinkGenerator { base_url })
    }
}
