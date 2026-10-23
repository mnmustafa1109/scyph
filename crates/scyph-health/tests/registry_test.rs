use scyph_health::{HealthFailure, HealthRegistry, Status, health_routes};
use tower::ServiceExt;

#[tokio::test]
async fn test_readyz_masks_details_by_default_and_verbose_reveals_them() {
    use axum::{Router, http::uri::Uri};

    let registry = HealthRegistry::new();
    registry
        .set(
            "database",
            Status::Sick,
            Some("connection refused: postgres.internal:5432".into()),
            true,
        )
        .await;

    let app: Router = health_routes(registry.clone());
    let masked = axum::body::to_bytes(
        app.oneshot(
            axum::extract::Request::builder()
                .uri(Uri::from_static("/readyz"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .into_body(),
        64 * 1024,
    )
    .await
    .unwrap();
    let masked = String::from_utf8_lossy(&masked);
    assert!(!masked.contains("connection refused"));
    assert!(masked.contains("Details hidden"));

    let verbose_app: Router = scyph_health::health_routes_verbose(registry);
    let revealed = axum::body::to_bytes(
        verbose_app
            .oneshot(
                axum::extract::Request::builder()
                    .uri(Uri::from_static("/readyz"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .into_body(),
        64 * 1024,
    )
    .await
    .unwrap();
    let revealed = String::from_utf8_lossy(&revealed);
    assert!(revealed.contains("connection refused"));
}

#[tokio::test]
async fn test_state_machine_transitions() {
    let registry = HealthRegistry::new();

    // 1. Initial healthy check
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot().await["db"].status, Status::Healthy);

    // 2. Transient failure -> Sick
    registry
        .check("db", true, async { Err::<(), &str>("Timeout") })
        .await;
    assert_eq!(registry.snapshot().await["db"].status, Status::Sick);

    // 3. First recovery probe -> Recovering
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot().await["db"].status, Status::Recovering);

    // 4. Second probe -> Healthy
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot().await["db"].status, Status::Healthy);

    // 5. Fatal failure -> Deceased
    registry
        .check("db", true, async {
            Err::<(), HealthFailure>(HealthFailure::Fatal("API key revoked".into()))
        })
        .await;
    assert_eq!(registry.snapshot().await["db"].status, Status::Deceased);
}
