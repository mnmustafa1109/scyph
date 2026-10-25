#[cfg(feature = "health")]
use scyph_health::{HealthRegistry, Status};

use std::time::Duration;

use scyph_db::DbHealthExt;
use sqlx::postgres::PgPoolOptions;

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

/// A server that accepts the TCP connection and then never answers must cost one
/// bounded probe: without the 5s bound, `SELECT 1` would wait on the connect/read
/// window and freeze the caller's sequential probe loop (and `/readyz`'s view of
/// every dependency behind it) for the whole stall.
#[tokio::test]
async fn probe_is_bounded_when_postgres_stalls() {
    let addr = scyph_test_support::spawn_hung_server().await;

    let pool = PgPoolOptions::new()
        .connect_lazy(&format!("postgres://postgres@{addr}/postgres"))
        .expect("loopback URL must parse");

    let registry = HealthRegistry::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        pool.check_health_named(&registry, "database", true),
    )
    .await
    .expect("a stalled server must cost one bounded probe, not hang the loop");

    let snap = registry.snapshot().await.get("database").unwrap().clone();
    assert!(
        matches!(snap.status, Status::Sick),
        "a stall may self-heal, so it is transient: {snap:?}"
    );
    assert!(
        snap.details
            .as_deref()
            .unwrap_or_default()
            .contains("timed out after 5s"),
        "the probe must be bounded by the 5s timeout, not another error: {:?}",
        snap.details
    );
}
