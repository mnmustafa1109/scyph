use scyph_auth::PasswordService;
use secrecy::SecretString;

#[tokio::test]
async fn test_password_service_hashing_and_verification() {
    let service = PasswordService::default();
    let password = SecretString::from("super_secret_password_123");

    let phc_hash = service.hash_password(&password).await.unwrap();
    assert!(phc_hash.starts_with("$argon2id$"));

    assert!(service.verify_password(&password, &phc_hash).await.is_ok());

    let wrong_password = SecretString::from("wrong_password");
    assert!(service.verify_password(&wrong_password, &phc_hash).await.is_err());
}
