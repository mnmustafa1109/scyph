use axum::{Router, http::HeaderName};
use tower_http::{
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

/// Wraps an [`axum::Router`] with standard telemetry, request ID propagation, dynamic compression, and HTTP tracing layers.
pub fn with_telemetry<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    let request_id_header = HeaderName::from_static("x-request-id");
    router
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
}
