use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use scyph_core::ApiResponse;
use scyph_telemetry::{
    init_tracing, with_telemetry, with_telemetry_config, RequestId, TelemetryConfig,
};
use tower::ServiceExt;

async fn sample_handler(RequestId(req_id): RequestId) -> String {
    format!("Request ID: {req_id}")
}

async fn json_handler() -> ApiResponse<String> {
    ApiResponse::ok("Privacy settings retrieved successfully".into())
}

#[tokio::test]
async fn test_telemetry_init_and_middleware() {
    let guard = init_tracing().expect("tracing init should succeed");
    assert!(guard.is_active());

    let app = Router::new().route("/test", get(sample_handler));
    let app = with_telemetry(app);

    let request = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));

    let req_id_header = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(!req_id_header.is_empty());
}

#[tokio::test]
async fn test_auto_meta_injection() {
    let app = Router::new().route("/api/data", get(json_handler));
    let app = with_telemetry(app);

    let request = Request::builder()
        .uri("/api/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let req_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["message"], "OK");
    assert_eq!(json["data"], "Privacy settings retrieved successfully");

    let meta = &json["meta"];
    assert!(meta.is_object());
    assert_eq!(meta["trace_id"], req_id);
    assert_eq!(meta["version"], env!("CARGO_PKG_VERSION"));
    assert!(meta["processing_time_ms"].is_number());
    assert!(meta["timestamp"].is_string());
}

#[tokio::test]
async fn test_telemetry_config_builder() {
    let config = TelemetryConfig::new()
        .with_api_version("2.5.0")
        .with_auto_meta(true)
        .with_tracing(true)
        .with_compression(false);

    let app = Router::new().route("/api/data", get(json_handler));
    let app = with_telemetry_config(app, config);

    let request = Request::builder()
        .uri("/api/data")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["meta"]["version"], "2.5.0");
}
