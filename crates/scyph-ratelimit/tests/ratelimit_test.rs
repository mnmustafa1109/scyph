use std::net::SocketAddr;
use std::time::Duration;
use axum::{body::Body, extract::ConnectInfo, http::{Request, StatusCode}, routing::get, Router};
use scyph_ratelimit::{per_ip_layer, relaxed_layer, strict_layer, RateLimitConfig, RateLimitRouterExt};
use tower::ServiceExt;

#[tokio::test]
async fn test_rate_limit_layer_instantiation() {
    let _per_ip = per_ip_layer::<Body>(10, Duration::from_secs(1)).unwrap();
    let _strict = strict_layer::<Body>().unwrap();
    let _relaxed = relaxed_layer::<Body>().unwrap();
    let _config_layer = RateLimitConfig::new()
        .with_burst(15)
        .with_period(Duration::from_secs(2))
        .build_layer::<Body>()
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_router_integration() {
    let app = Router::new()
        .route("/api/ping", get(|| async { "pong" }))
        .layer(relaxed_layer().unwrap());

    let mut request = Request::builder()
        .uri("/api/ping")
        .body(Body::empty())
        .unwrap();

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(addr));

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_router_ext() {
    let app = Router::new()
        .route("/api/hello", get(|| async { "hello" }))
        .rate_limit_relaxed();

    let mut request = Request::builder()
        .uri("/api/hello")
        .body(Body::empty())
        .unwrap();

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(addr));

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
