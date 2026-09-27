//! Telemetry middleware layers, UUIDv7 request ID generation, and Axum extractors.

use std::fmt::{Display, Formatter};

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderName, HeaderValue, Request},
    Router,
};
use scyph_core::AppError;
use tower_http::{
    compression::CompressionLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId as TowerRequestId, SetRequestIdLayer},
    trace::TraceLayer,
};
use uuid::Uuid;

/// Generator producing time-ordered UUIDv7 strings for incoming HTTP request IDs.
#[derive(Clone, Copy, Debug, Default)]
pub struct MakeRequestIdV7;

impl MakeRequestId for MakeRequestIdV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<TowerRequestId> {
        let uuid_str = Uuid::now_v7().to_string();
        let header_val = HeaderValue::from_str(&uuid_str).ok()?;
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
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            request_id_header: HeaderName::from_static("x-request-id"),
            enable_compression: true,
            enable_tracing: true,
        }
    }
}

/// Wraps an [`axum::Router`] with standard telemetry, UUIDv7 request ID propagation, dynamic compression, and tracing layers.
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

    let header_name = config.request_id_header;
    router
        .layer(PropagateRequestIdLayer::new(header_name.clone()))
        .layer(SetRequestIdLayer::new(header_name, MakeRequestIdV7))
}
