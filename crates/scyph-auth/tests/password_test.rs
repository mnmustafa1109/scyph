use scyph_auth::PasswordService;
use secrecy::SecretString;

/// Fast cost parameters so pepper round-trips don't dominate CI.
fn fast_service(pepper: Option<&str>) -> PasswordService {
    let service = PasswordService::new(2, 2048, 2, 1).unwrap();
    match pepper {
        Some(secret) => service.with_secret(SecretString::from(secret.to_string())),
        None => service,
    }
}

#[tokio::test]
async fn test_password_service_hashing_and_verification() {
    let service = PasswordService::default();
    let password = SecretString::from("super_secret_password_123");

    let phc_hash = service.hash_password(&password).await.unwrap();
    assert!(phc_hash.starts_with("$argon2id$"));

    assert!(service.verify_password(&password, &phc_hash).await.is_ok());

    let wrong_password = SecretString::from("wrong_password");
    assert!(
        service
            .verify_password(&wrong_password, &phc_hash)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn test_peppered_hash_verifies_only_with_same_pepper() {
    let password = SecretString::from("pepper_independence_test");

    let service = fast_service(Some("pepper:correct"));
    let hash = service.hash_password(&password).await.unwrap();

    // Same pepper verifies; a different or absent pepper must fail.
    assert!(service.verify_password(&password, &hash).await.is_ok());
    assert!(
        fast_service(Some("pepper:wrong"))
            .verify_password(&password, &hash)
            .await
            .is_err()
    );
    assert!(
        fast_service(None)
            .verify_password(&password, &hash)
            .await
            .is_err()
    );

    // Peppered and unpeppered hashes for the same password differ.
    let unpeppered = fast_service(None).hash_password(&password).await.unwrap();
    assert_ne!(hash, unpeppered);
}
