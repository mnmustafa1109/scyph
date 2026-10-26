use scyph_utils::bloom::{ValkeyBloomError, ValkeyBloomFilter};

// This file intentionally contains a single test, so mutating the process
// environment cannot race with another test in this binary.
#[test]
fn from_env_requires_a_url() {
    // SAFETY: no other thread in this test binary reads or writes the environment.
    unsafe {
        std::env::remove_var("VALKEY_URL");
        std::env::remove_var("REDIS_URL");
    }

    let err = ValkeyBloomFilter::from_env("crm", 100, 0.01).unwrap_err();
    assert!(matches!(err, ValkeyBloomError::Config(_)));
}
