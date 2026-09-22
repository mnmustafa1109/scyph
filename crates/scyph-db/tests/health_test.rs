#[cfg(feature = "health")]
use scyph_health::{HealthRegistry, Status};

#[tokio::test]
async fn test_db_health_ext_mock() {
    let registry = HealthRegistry::new();

    // Verify initial health registry state is empty
    assert!(registry.snapshot().await.is_empty());

    // Set custom database status
    registry.set("database", Status::Healthy, None, true).await;

    assert_eq!(
        registry.snapshot().await["database"].status,
        Status::Healthy
    );
}
