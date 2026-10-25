//! Covers the missing-variable branch of `jwt_secret_from_env`.
//!
//! Separate file on purpose: this test *removes* `JWT_SECRET` while
//! `jwt_test.rs` *sets* it, and each integration test file is its own
//! process — so the two can never race on the environment.

use scyph_auth::jwt_secret_from_env;

#[test]
fn jwt_secret_from_env_requires_the_variable() {
    // Safe: no other test in this binary reads or writes `JWT_SECRET`.
    unsafe { std::env::remove_var("JWT_SECRET") };
    let err = jwt_secret_from_env().expect_err("an unset JWT_SECRET must be rejected at boot");
    assert!(
        err.to_string().contains("must be set"),
        "unexpected error: {err}"
    );
}
