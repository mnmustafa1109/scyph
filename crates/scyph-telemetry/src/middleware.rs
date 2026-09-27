//! Telemetry middleware layers, UUIDv7 request ID generation, and Axum extractors.

use std::fmt::{Display, Formatter};

use axum::{
    Router,
    extract::FromRequestParts,
    http::{HeaderName, HeaderValue, Request, request::Parts},
    response::Response,
};
use scyph_core::{AppError, ResponseMeta};
use tower_http::{
    compression::CompressionLayer,
    request_id::{
        MakeRequestId, PropagateRequestIdLayer, RequestId as TowerRequestId, SetRequestIdLayer,
    },
    trace::TraceLayer,
};
use uuid::Uuid;

/// Generator producing time-ordered UUIDv7 strings for incoming HTTP request IDs.
#[derive(Clone, Copy, Debug, Default)]
pub struct MakeRequestIdV7;

impl MakeRequestId for MakeRequestIdV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<TowerRequestId> {
        let mut buf = [0u8; 36];
        let uuid_str = Uuid::now_v7().as_hyphenated().encode_lower(&mut buf);
        let header_val = HeaderValue::from_str(uuid_str).ok()?;
        Some(TowerRequestId::new(header_val))
    }
}

/// Type-safe Axum extractor for the current HTTP request ID.
///
/// Extracts request ID from the `tower_http::request_id::RequestId` request extension or `x-request-id` header.
/// Falls back to generating a new UUIDv7 if no request ID header is present.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::response::IntoResponse;
/// use scyph_telemetry::middleware::RequestId;
///
/// async fn handler(RequestId(req_id): RequestId) -> impl IntoResponse {
///     format!("Request ID: {req_id}")
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(pub String);

impl RequestId {
    /// Exposes a string slice reference to the request ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for RequestId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(str_val) = parts
            .extensions
            .get::<TowerRequestId>()
            .and_then(|id| id.header_value().to_str().ok())
        {
            return Ok(RequestId(str_val.to_string()));
        }

        if let Some(str_val) = parts
            .headers
            .get("x-request-id")
            .and_then(|header_val| header_val.to_str().ok())
        {
            return Ok(RequestId(str_val.to_string()));
        }

        Ok(RequestId(Uuid::now_v7().to_string()))
    }
}

/// Configuration options for custom HTTP telemetry layers.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Custom HTTP header name used for request ID propagation (default: `"x-request-id"`).
    pub request_id_header: HeaderName,
    /// Enables dynamic response payload compression (gzip, brotli, zstd) (default: `true`).
    pub enable_compression: bool,
    /// Enables structured HTTP tracing spans (default: `true`).
    pub enable_tracing: bool,
    /// Enables automatic injection of standard `ResponseMeta` into JSON response envelopes (default: `true`).
    pub enable_auto_meta: bool,
    /// Application API version string injected into `ResponseMeta` (default: `"0.1.0"`).
    pub api_version: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            request_id_header: HeaderName::from_static("x-request-id"),
            enable_compression: true,
            enable_tracing: true,
            enable_auto_meta: true,
            api_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl TelemetryConfig {
    /// Constructs a new [`TelemetryConfig`] builder with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP header name for request ID propagation.
    pub fn with_request_id_header(mut self, header: HeaderName) -> Self {
        self.request_id_header = header;
        self
    }

    /// Enables or disables response compression.
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Enables or disables tracing spans.
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// Enables or disables automatic `ResponseMeta` JSON injection.
    pub fn with_auto_meta(mut self, enable: bool) -> Self {
        self.enable_auto_meta = enable;
        self
    }

    /// Sets the application API version string for `ResponseMeta`.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }
}

/// Axum middleware for automatically injecting standard [`ResponseMeta`] into JSON responses.
///
/// Measures request latency, retrieves the request trace ID, and populates the `"meta"` field in response envelopes.
pub async fn auto_meta_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
    config: TelemetryConfig,
) -> Response {
    let start = std::time::Instant::now();
    let req_header = config.request_id_header.clone();
    let trace_id = request
        .headers()
        .get(&req_header)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::now_v7().to_string());

    let response = next.run(request).await;

    if !config.enable_auto_meta {
        return response;
    }

    let is_json = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("json"))
        .unwrap_or(false);

    if !is_json {
        return response;
    }

    let processing_time_ms = start.elapsed().as_millis() as u64;
    let (parts, body) = response.into_parts();

    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return Response::from_parts(parts, axum::body::Body::empty()),
    };

    if let Ok(mut json_val) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && let Some(obj) = json_val.as_object_mut()
    {
        if !obj.contains_key("meta") || obj.get("meta").is_none_or(|v| v.is_null()) {
            let meta = ResponseMeta::new(trace_id, processing_time_ms, &config.api_version);
            if let Ok(meta_val) = serde_json::to_value(meta) {
                obj.insert("meta".to_string(), meta_val);
            }
        }
        if let Ok(new_bytes) = serde_json::to_vec(&json_val) {
            let mut parts = parts;
            parts.headers.insert(
                axum::http::header::CONTENT_LENGTH,
                HeaderValue::from(new_bytes.len()),
            );
            return Response::from_parts(parts, axum::body::Body::from(new_bytes));
        }
    }

    Response::from_parts(parts, axum::body::Body::from(bytes))
}

/// Wraps an [`axum::Router`] with standard telemetry, UUIDv7 request ID propagation, dynamic compression, tracing, and auto-meta response injection.
///
/// Uses default [`TelemetryConfig`].
pub fn with_telemetry<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    with_telemetry_config(router, TelemetryConfig::default())
}

/// Wraps an [`axum::Router`] with customized telemetry layers defined by [`TelemetryConfig`].
pub fn with_telemetry_config<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    config: TelemetryConfig,
) -> Router<S> {
    let mut router = router;

    if config.enable_compression {
        router = router.layer(CompressionLayer::new());
    }

    if config.enable_tracing {
        let req_header = config.request_id_header.clone();
        let trace_layer = TraceLayer::new_for_http().make_span_with(move |request: &Request<_>| {
            let req_id = request
                .headers()
                .get(&req_header)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
                request_id = %req_id,
            )
        });
        router = router.layer(trace_layer);
    }

    if config.enable_auto_meta {
        let auto_meta_config = config.clone();
        router = router.layer(axum::middleware::from_fn(move |req, next| {
            auto_meta_middleware(req, next, auto_meta_config.clone())
        }));
    }

    let header_name = config.request_id_header;
    router
        .layer(PropagateRequestIdLayer::new(header_name.clone()))
        .layer(SetRequestIdLayer::new(header_name, MakeRequestIdV7))
}
