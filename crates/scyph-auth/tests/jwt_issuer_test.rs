//! Covers the `JWT_ISSUER` branch of `create_token`/`verify_token`: setting it
//! stamps an `iss` claim and makes verification require it. Lives in its own
//! test binary so the process-wide env var cannot race other token tests.

use scyph_auth::{create_token, verify_token};
use scyph_core::Claims;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
struct MinimalClaims {
    sub: Uuid,
    exp: i64,
    jti: String,
}

impl Claims for MinimalClaims {
    type Role = ();
    fn subject(&self) -> Uuid {
        self.sub
    }
    fn expiry(&self) -> i64 {
        self.exp
    }
    fn jti(&self) -> &str {
        &self.jti
    }
    fn role(&self) -> &Self::Role {
        &()
    }
}

fn secret() -> SecretString {
    SecretString::from("at-least-32-bytes-long-secret-key!!")
}

fn claims() -> MinimalClaims {
    MinimalClaims {
        sub: Uuid::now_v7(),
        exp: chrono::Utc::now().timestamp() + 3600,
        jti: Uuid::now_v7().to_string(),
    }
}

/// With `JWT_ISSUER` set: signing stamps `iss`, verification with the same
/// issuer passes, a different issuer rejects, and unsetting the variable keeps
/// outstanding tokens verifiable. Without it: the pre-issuer contract holds
/// (no `iss` stamped, no `iss` required).
#[test]
fn jwt_issuer_is_stamped_and_enforced() {
    // Safety: this test binary owns JWT_ISSUER; cargo runs test binaries
    // sequentially, so no other test observes this process's env.
    unsafe {
        std::env::set_var("JWT_ISSUER", "lansan");
        let secret = secret();
        let claims = claims();

        let token = create_token(&claims, &secret).expect("signing with issuer failed");
        let decoded = verify_token::<MinimalClaims>(&token, &secret)
            .expect("a token signed with the issuer must verify");
        assert_eq!(decoded.claims.subject(), claims.sub);

        std::env::set_var("JWT_ISSUER", "someone-else");
        assert!(
            verify_token::<MinimalClaims>(&token, &secret).is_err(),
            "a token from a different issuer must be rejected"
        );

        std::env::remove_var("JWT_ISSUER");
        verify_token::<MinimalClaims>(&token, &secret)
            .expect("with no issuer configured, existing tokens must still verify");

        let plain_token = create_token(&claims, &secret).expect("signing without issuer failed");
        verify_token::<MinimalClaims>(&plain_token, &secret)
            .expect("without JWT_ISSUER, tokens carry no iss requirement");
    }
}
