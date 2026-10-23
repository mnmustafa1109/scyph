//! End-to-end tests for [`RoleRouterExt::require_roles`] layer ordering.
//!
//! Guards the `.route_layer(...)` -> `.layer(Extension(AllowedRoles(...)))` ordering: the
//! `Extension` must be outermost (added last) so it inserts `AllowedRoles` into the request
//! *before* the role-check middleware extracts it. The order was once inverted and every
//! protected route failed with HTTP 500 (`MissingExtension`) — this file prevents a silent
//! regression in authz logic that no compiler or linter can catch.

use axum::{
    Router,
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::get,
};
use scyph_auth::{AuthCacheService, AuthExtractorState, RoleRouterExt, create_token};
use scyph_core::Claims;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum TestRole {
    Viewer,
    Admin,
}

// Authorizable is a marker trait (Copy + Eq + Send + Sync) — no impl body needed.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    jti: String,
    role: TestRole,
}

impl Claims for TestClaims {
    type Role = TestRole;

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
        &self.role
    }
}

#[derive(Clone)]
struct TestState {
    secret: SecretString,
    cache: Arc<AuthCacheService<()>>,
}

impl AuthExtractorState<TestClaims> for TestState {
    type CachePool = ();

    fn jwt_secret(&self) -> &SecretString {
        &self.secret
    }

    fn auth_cache(&self) -> &Arc<AuthCacheService<()>> {
        &self.cache
    }
}

fn test_state() -> TestState {
    TestState {
        secret: SecretString::from("test_jwt_secret_only_for_tests"),
        cache: Arc::new(AuthCacheService::new(128, 60, 60)),
    }
}

fn protected_router(state: &TestState) -> Router {
    Router::new()
        .route("/admin", get(|| async { "admin ok" }))
        .require_roles::<TestClaims>(state.clone(), &[TestRole::Admin])
        .with_state(state.clone())
}

fn claims(role: TestRole) -> TestClaims {
    TestClaims {
        sub: Uuid::now_v7(),
        exp: chrono::Utc::now().timestamp() + 300,
        jti: format!("jti-{role:?}"),
        role,
    }
}

/// Issues a request to the protected route and returns the HTTP status.
async fn status_for(state: &TestState, route: &str, token: Option<&str>) -> StatusCode {
    let app = protected_router(state);
    let mut req = Request::builder().uri(route);
    if let Some(token) = token {
        req = req.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = app
        .oneshot(req.body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    response.status()
}

#[tokio::test]
async fn allowed_role_gets_200_below_route() {
    let state = test_state();
    let secret = state.jwt_secret();
    let token = create_token(&claims(TestRole::Admin), secret).unwrap();
    assert_eq!(
        status_for(&state, "/admin", Some(&token)).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn wrong_role_gets_403_forbidden() {
    let state = test_state();
    let secret = state.jwt_secret();
    let token = create_token(&claims(TestRole::Viewer), secret).unwrap();
    assert_eq!(
        status_for(&state, "/admin", Some(&token)).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn without_token_gets_401_unauthorized() {
    let state = test_state();
    assert_eq!(
        status_for(&state, "/admin", None).await,
        StatusCode::UNAUTHORIZED
    );
}

/// A later-added route must NOT be protected (`require_roles` applies only to routes
/// configured before the call).
#[tokio::test]
async fn unprotected_route_served_without_auth() {
    let state = test_state();
    let app: Router = Router::new()
        .route("/admin", get(|| async { "admin ok" }))
        .require_roles::<TestClaims>(state.clone(), &[TestRole::Admin])
        .with_state(state.clone())
        .route("/public", get(|| async { "public ok" }));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/public")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
