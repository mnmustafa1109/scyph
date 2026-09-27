use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use scyph_telemetry::{init_tracing, with_telemetry, RequestId};
use tower::ServiceExt;

async fn sample_handler(RequestId(req_id): RequestId) -> String {
    format!("Request ID: {req_id}")
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
