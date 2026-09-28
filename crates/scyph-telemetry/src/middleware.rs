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

/// Axum middleware for injecting telemetry headers (`x-response-time-ms`, `x-api-version`, `x-trace-id`)
/// on all responses, and automatically injecting standard [`ResponseMeta`] into JSON response bodies.
///
/// Measures request latency, retrieves or generates the request trace ID, attaches HTTP telemetry
/// headers to every outgoing response, and safely injects `"meta"` into discrete `application/json` payloads
/// without breaking streaming (SSE, ndjson) or dropping bodies.
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

    let mut response = next.run(request).await;
    let processing_time_ms = start.elapsed().as_millis() as u64;

    // Solution B: Always inject telemetry headers on the outgoing response.
    // Guarantees non-JSON (HTML, raw text, images), streaming SSE, downloads, and errors
    // all receive complete telemetry metadata headers without buffering or touching the body.
    let headers = response.headers_mut();
    if let Ok(val) = HeaderValue::from_str(&processing_time_ms.to_string()) {
        headers.insert(HeaderName::from_static("x-response-time-ms"), val);
    }
    if let Ok(val) = HeaderValue::from_str(&config.api_version) {
        headers.insert(HeaderName::from_static("x-api-version"), val);
    }
    if let Ok(val) = HeaderValue::from_str(&trace_id) {
        headers
            .entry(HeaderName::from_static("x-trace-id"))
            .or_insert_with(|| val.clone());
        headers
            .entry(config.request_id_header.clone())
            .or_insert(val);
    }

    if !config.enable_auto_meta {
        return response;
    }

    // Solution A - Guard 1: Strict Content-Type check.
    // Only intercept discrete `application/json` or `application/problem+json`.
    // Explicitly avoids `text/event-stream`, `application/x-ndjson`, `application/json-seq`, etc.
    let is_discrete_json = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            let ct_lower = ct.to_ascii_lowercase();
            let mime = ct_lower.split(';').next().unwrap_or("").trim();
            mime == "application/json" || mime == "application/problem+json"
        })
        .unwrap_or(false);

    if !is_discrete_json {
        return response;
    }

    // Solution A - Guard 2: Skip chunked or streaming transfer-encodings.
    if response
        .headers()
        .contains_key(axum::http::header::TRANSFER_ENCODING)
    {
        return response;
    }

    // Solution A - Guard 3: Skip large payloads (> 2MB) to prevent latency spikes and memory churn.
    // Large responses pass through untouched — the body is NEVER dropped!
    const MAX_META_BODY_BYTES: usize = 2 * 1024 * 1024;
    if let Some(content_length) = response
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        && content_length > MAX_META_BODY_BYTES
    {
        return response;
    }

    // Solution A - Guard 4 & Zero-Loss fallback:
    let (parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_META_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => {
            return Response::from_parts(parts, axum::body::Body::empty());
        }
    };

    // Fast path: if "meta" key is already present in the JSON body, do not re-serialize!
    let already_has_meta = {
        let pattern = b"\"meta\"";
        bytes.windows(pattern.len()).any(|w| w == pattern)
    };

    if already_has_meta
        && let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && let Some(obj) = json_val.as_object()
        && obj.get("meta").is_some_and(|v| !v.is_null())
    {
        return Response::from_parts(parts, axum::body::Body::from(bytes));
    }

    // Safely inject "meta" if the root is a JSON object
    if let Ok(mut json_val) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && let Some(obj) = json_val.as_object_mut()
    {
        let meta = ResponseMeta::new(trace_id, processing_time_ms, &config.api_version);
        if let Ok(meta_val) = serde_json::to_value(meta) {
            obj.insert("meta".to_string(), meta_val);
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

    // Zero-loss fallback: return original bytes untouched
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

    // 1. auto_meta_middleware (Innermost response layer: injects meta & telemetry headers on uncompressed body)
    if config.enable_auto_meta {
        let auto_meta_config = config.clone();
        router = router.layer(axum::middleware::from_fn(move |req, next| {
            auto_meta_middleware(req, next, auto_meta_config.clone())
        }));
    }

    // 2. Tracing spans
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

    // 3. CompressionLayer (Outermost response layer: compresses finalized response body)
    if config.enable_compression {
        router = router.layer(CompressionLayer::new());
    }

    // 4. Request ID propagation and generation
    let header_name = config.request_id_header;
    router
        .layer(PropagateRequestIdLayer::new(header_name.clone()))
        .layer(SetRequestIdLayer::new(header_name, MakeRequestIdV7))
}
