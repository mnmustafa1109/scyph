//! Integration tests for JWT secret configuration (`jwt_secret_from_env`).
//!
//! Tested through the public `jwt_secret_from_env()` because the length
//! validator is a crate-private helper. One test function only: it mutates the
//! process-global `JWT_SECRET` variable, and environment variables cannot be
//! set per-thread.

use scyph_auth::jwt_secret_from_env;

#[test]
fn jwt_secret_from_env_enforces_256_bits() {
    // Safe: no other test in this binary reads or writes `JWT_SECRET`.
    unsafe { std::env::set_var("JWT_SECRET", "short") };
    let err = jwt_secret_from_env().expect_err("a short secret must be rejected");
    assert!(
        err.to_string().contains("32 bytes"),
        "unexpected error: {err}"
    );

    unsafe { std::env::set_var("JWT_SECRET", "x".repeat(31)) };
    assert!(
        jwt_secret_from_env().is_err(),
        "a 31-byte secret must be rejected"
    );

    unsafe { std::env::set_var("JWT_SECRET", "x".repeat(32)) };
    assert!(
        jwt_secret_from_env().is_ok(),
        "a 32-byte secret must be accepted"
    );
}
