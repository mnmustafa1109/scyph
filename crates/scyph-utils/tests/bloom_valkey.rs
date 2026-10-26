use scyph_utils::bloom::ValkeyBloomFilter;

fn env_url() -> String {
    std::env::var("VALKEY_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

#[test]
fn key_includes_sizing() {
    let url = "redis://127.0.0.1:6379";
    let a = ValkeyBloomFilter::new(url, "crm", 100, 0.01).unwrap();
    let b = ValkeyBloomFilter::new(url, "crm", 1_000_000, 0.01).unwrap();
    let c = ValkeyBloomFilter::new(url, "crm", 100, 0.01).unwrap();
    assert_ne!(a.key(), b.key());
    assert_eq!(a.key(), c.key());
    assert!(a.key().contains(":bloom:v1:"));
}

#[tokio::test]
#[ignore = "requires a running Valkey/Redis instance"]
async fn round_trip() {
    let url = env_url();
    let filter = ValkeyBloomFilter::new(&url, "scyph:bloom:test", 100, 0.01).unwrap();
    filter.ping().await.unwrap();
    filter.clear().await.unwrap();
    assert!(!filter.contains(&42i32).await.unwrap());
    assert!(!filter.add(&42i32).await.unwrap());
    assert!(filter.contains(&42i32).await.unwrap());

    filter.add_many([1i32, 2, 3]).await.unwrap();
    assert!(filter.contains(&1).await.unwrap());

    let warm = ValkeyBloomFilter::from_items(&url, "scyph:bloom:test2", 0.01, [4i32, 5])
        .await
        .unwrap();
    assert!(warm.contains(&4).await.unwrap());
    warm.clear().await.unwrap();

    let sized =
        ValkeyBloomFilter::from_items_with_size(&url, "scyph:bloom:test3", 1_000, 0.01, [6i32, 7])
            .await
            .unwrap();
    assert!(sized.contains(&6).await.unwrap());
    assert!(sized.key().contains(":bloom:v1:"));
    sized.clear().await.unwrap();
    filter.clear().await.unwrap();
}
