//! Covers the 5s timeout arm of `S3StorageService::from_env()`.
//!
//! Separate file (own process): it mutates the S3/AWS environment variables,
//! which the tests in `s3_config_test.rs` also use — separate binaries cannot
//! share them. All endpoints point at loopback.

use scyph_storage::S3StorageService;

/// The endpoint accepts the TCP connection but never answers, so the request
/// hangs until `from_env`'s 5s timeout fires.
#[tokio::test]
async fn from_env_times_out_when_bucket_never_answers() {
    let addr = scyph_test_support::spawn_hung_server().await;

    // Safe: no other test in this binary reads or writes these variables.
    unsafe {
        std::env::set_var("S3_BUCKET", "boot-check-bucket");
        std::env::set_var("S3_ENDPOINT_URL", format!("http://{addr}"));
        std::env::set_var("AWS_ACCESS_KEY_ID", "test-key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret");
        std::env::remove_var("S3_PUBLIC_URL_PREFIX");
        std::env::remove_var("S3_REGION");
        std::env::remove_var("AWS_REGION");
    }

    let err = S3StorageService::from_env()
        .await
        .err()
        .expect("an endpoint that never answers must fail the boot check");
    assert!(
        err.to_string().contains("timed out after 5s"),
        "unexpected error: {err}"
    );
    assert!(
        err.to_string().contains(&addr.to_string()),
        "the timeout error must name the endpoint it contacted: {err}"
    );
}
