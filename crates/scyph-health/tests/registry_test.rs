use scyph_health::{HealthFailure, HealthRegistry, Status};

#[tokio::test]
async fn test_state_machine_transitions() {
    let registry = HealthRegistry::new();

    // 1. Initial healthy check
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot()["db"].status, Status::Healthy);
    assert!(registry.is_ready());

    // 2. Transient failure -> Sick
    registry
        .check("db", true, async { Err::<(), &str>("Timeout") })
        .await;
    assert_eq!(registry.snapshot()["db"].status, Status::Sick);
    assert!(!registry.is_ready());

    // 3. First recovery probe -> Recovering
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot()["db"].status, Status::Recovering);
    assert!(!registry.is_ready());

    // 4. Second probe -> Healthy
    registry
        .check("db", true, async { Ok::<(), &str>(()) })
        .await;
    assert_eq!(registry.snapshot()["db"].status, Status::Healthy);
    assert!(registry.is_ready());

    // 5. Fatal failure -> Deceased
    registry
        .check("db", true, async {
            Err::<(), HealthFailure>(HealthFailure::Fatal("API key revoked".into()))
        })
        .await;
    assert_eq!(registry.snapshot()["db"].status, Status::Deceased);
    assert!(!registry.is_ready());

    // 6. Manual synchronous set
    registry.set("cache", Status::Healthy, None, false);
    assert_eq!(registry.snapshot()["cache"].status, Status::Healthy);
}
