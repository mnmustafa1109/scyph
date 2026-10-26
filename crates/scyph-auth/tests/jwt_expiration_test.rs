//! Covers `jwt_expiration_from_env`: unset ⇒ 24h default, set ⇒ parsed seconds,
//! invalid or non-positive ⇒ `JwtError::Config`. Own test binary so the
//! process-wide env var cannot race other tests.

use scyph_auth::{JwtError, jwt_expiration_from_env};

#[test]
fn jwt_expiration_is_env_configurable_with_safe_fallback() {
    // Safety: this test binary owns JWT_EXPIRATION; cargo runs test binaries
    // sequentially, so no other test observes this process's env.
    unsafe {
        std::env::remove_var("JWT_EXPIRATION");
        assert_eq!(jwt_expiration_from_env().unwrap(), 86_400);

        std::env::set_var("JWT_EXPIRATION", "3600");
        assert_eq!(jwt_expiration_from_env().unwrap(), 3600);

        for bad in ["abc", "0", "-5", ""] {
            std::env::set_var("JWT_EXPIRATION", bad);
            assert!(
                matches!(jwt_expiration_from_env(), Err(JwtError::Config(_))),
                "JWT_EXPIRATION={bad:?} must fail config validation"
            );
        }
        std::env::remove_var("JWT_EXPIRATION");
    }
}
