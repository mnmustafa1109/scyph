use scyph_health::{HealthFailure, HealthRegistry, Status};

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
