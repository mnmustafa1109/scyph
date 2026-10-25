//! Boot-time validation tests for [`S3StorageService::from_env()`].
//!
//! The env-mutating tests below mutate the same process-global S3/AWS environment
//! variables, so they serialize on `ENV_LOCK`. All endpoints point at loopback, so
//! no network access is required.

// Only the health-probe tests below need the hung-server helper; without the `health`
// feature this binary compiles nothing that uses it (and `clippy -D warnings` rejects
// an unused helper).
#[cfg(feature = "health")]
mod common;

use scyph_storage::{S3StorageService, sanitize_endpoint};

/// Serializes the env-mutating tests below (environment variables are process-global).
/// Async mutex: the guard is held across `.await` on purpose, so it must not be a `std::sync::Mutex`.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn set_s3_env(endpoint: &str) {
    // Safe: only this test binary touches these variables, and `ENV_LOCK` keeps
    // the env-mutating tests from interleaving their writes and reads.
    unsafe {
        std::env::set_var("S3_BUCKET", "boot-check-bucket");
        std::env::set_var("S3_ENDPOINT_URL", endpoint);
        std::env::set_var("AWS_ACCESS_KEY_ID", "test-key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret");
        std::env::remove_var("S3_PUBLIC_URL_PREFIX");
        std::env::remove_var("S3_REGION");
        std::env::remove_var("AWS_REGION");
    }
}

/// S3 error bodies for `ListObjectsV2`, which — unlike `HeadBucket` — carries a `<Code>`
/// the classification in `test_connection()` reads.
const ACCESS_DENIED: &str =
    "<Error><Code>AccessDenied</Code><Message>Access Denied</Message></Error>";
const INVALID_ACCESS_KEY: &str = "<Error><Code>InvalidAccessKeyId</Code>\
                                  <Message>The AWS Access Key Id you provided does not exist.\
                                  </Message></Error>";

/// Minimal loopback S3 stand-in answering every request with the given status and body —
/// what real S3 returns: 403 `AccessDenied` for a policy without `s3:ListBucket`, 403
/// `InvalidAccessKeyId` for wrong credentials, 404 for a missing bucket. An empty body
/// is what a gateway or an S3-compatible server answers when it sends no error XML.
async fn spawn_status_server(
    status_line: &'static str,
    body: &'static str,
) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        while let Ok((mut sock, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 {status_line}\r\ncontent-type: application/xml\r\ncontent-length: \
                     {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(response.as_bytes()).await;
            });
        }
    });
    addr
}

/// `from_env` must refuse to boot on an unreachable bucket rather than hand back
/// a client that only fails on the first upload.
#[tokio::test]
async fn from_env_fails_fast_when_bucket_unreachable() {
    let _env = ENV_LOCK.lock().await;
    // Port 9 (discard) on loopback: nothing listens, the connection is refused.
    set_s3_env("http://127.0.0.1:9");

    let err = S3StorageService::from_env()
        .await
        .err()
        .expect("an unreachable endpoint must fail at boot");
    assert!(
        err.to_string().contains("S3 boot check"),
        "unexpected error: {err}"
    );
}

/// A `403 Forbidden` with no error body — a gateway or an S3-compatible server that sends
/// no error XML — still means the bucket answered, so boot continues.
#[tokio::test]
async fn from_env_boots_on_403_without_error_code() {
    let _env = ENV_LOCK.lock().await;
    let addr = spawn_status_server("403 Forbidden", "").await;

    set_s3_env(&format!("http://{addr}"));

    S3StorageService::from_env()
        .await
        .expect("a 403 with no error code must not block boot");
}

/// A `403 AccessDenied` means the bucket answered: a least-privilege policy without
/// `s3:ListBucket` must not block boot.
#[tokio::test]
async fn from_env_boots_on_access_denied() {
    let _env = ENV_LOCK.lock().await;
    let addr = spawn_status_server("403 Forbidden", ACCESS_DENIED).await;

    set_s3_env(&format!("http://{addr}"));

    S3StorageService::from_env()
        .await
        .expect("a reachable bucket must boot even when credentials lack s3:ListBucket");
}

/// A 403 that names the credentials is a configuration typo, not a permission nuance:
/// boot must fail loudly instead of handing out a client whose uploads will all be rejected.
#[tokio::test]
async fn from_env_fails_fast_on_invalid_credentials() {
    let _env = ENV_LOCK.lock().await;
    let addr = spawn_status_server("403 Forbidden", INVALID_ACCESS_KEY).await;

    set_s3_env(&format!("http://{addr}"));

    let err = S3StorageService::from_env()
        .await
        .err()
        .expect("rejected credentials must fail at boot");
    assert!(
        err.to_string().contains("InvalidAccessKeyId"),
        "the error must name the rejected credential: {err}"
    );
}

#[test]
fn sanitize_endpoint_strips_trailing_bucket_and_slashes() {
    let bucket = "my-bucket";

    // Path-style (e.g. MinIO): strips trailing bucket and trailing slashes
    assert_eq!(
        sanitize_endpoint("http://localhost:9000", bucket, true),
        "http://localhost:9000"
    );
    assert_eq!(
        sanitize_endpoint("http://localhost:9000/", bucket, true),
        "http://localhost:9000"
    );
    assert_eq!(
        sanitize_endpoint("http://localhost:9000/my-bucket", bucket, true),
        "http://localhost:9000"
    );
    assert_eq!(
        sanitize_endpoint("http://localhost:9000/my-bucket/", bucket, true),
        "http://localhost:9000"
    );

    // Virtual-hosted AWS endpoint: preserved
    assert_eq!(
        sanitize_endpoint("https://s3.us-east-1.amazonaws.com", bucket, false),
        "https://s3.us-east-1.amazonaws.com"
    );
    assert_eq!(
        sanitize_endpoint("https://cdn.example.com/", bucket, false),
        "https://cdn.example.com"
    );
}

/// `normalize_key` must accept raw keys, full path-style URLs, presigned URLs
/// (query stripped) and keys that collide with the bucket name.
#[tokio::test]
async fn normalize_key_variations() {
    let service = S3StorageService::new(
        "my-bucket".into(),
        "http://localhost:9000".into(),
        "http://localhost:9000".into(),
        "us-east-1".into(),
        "test-key".into(),
        "test-secret".into(),
    )
    .await;

    // Raw key
    assert_eq!(
        service.normalize_key("uploads/photo.png"),
        "uploads/photo.png"
    );
    assert_eq!(
        service.normalize_key("/uploads/photo.png"),
        "uploads/photo.png"
    );

    // MinIO path-style URL with bucket
    assert_eq!(
        service.normalize_key("http://localhost:9000/my-bucket/uploads/photo.png"),
        "uploads/photo.png"
    );

    // MinIO path-style URL with query parameters
    assert_eq!(
        service
            .normalize_key("http://localhost:9000/my-bucket/uploads/photo.png?X-Amz-Signature=123"),
        "uploads/photo.png"
    );

    // Bucket-prefix collision: key segment that starts with the bucket name must survive
    assert_eq!(
        service.normalize_key("http://localhost:9000/my-bucket/database/photo.png"),
        "database/photo.png"
    );
    assert_eq!(
        service.normalize_key("database/photo.png"),
        "database/photo.png"
    );
}

/// Pins S3 health-probe failure classification: a missing bucket needs an operator
/// (`Deceased`), a 403 counts as reachable (`Healthy`, same decision boot makes),
/// and network failures may self-recover (`Sick`).
#[cfg(feature = "health")]
mod health_classification {
    use super::*;
    use scyph_health::{HealthRegistry, Status};
    use scyph_storage::StorageHealthExt;

    async fn storage_at(endpoint: String) -> S3StorageService {
        S3StorageService::new(
            "boot-check-bucket".into(),
            endpoint.clone(),
            endpoint,
            "us-east-1".into(),
            "test-key".into(),
            "test-secret".into(),
        )
        .await
    }

    /// `from_env()` deliberately boots on 403 (a least-privilege policy without
    /// `s3:ListBucket` still uploads fine), and `is_ready()` rejects any non-`Healthy`
    /// required component (`Sick`/`Recovering` included) — so the probe must count a
    /// 403 bucket as ready or it pins `/readyz` at 503 for a healthy deployment.
    #[tokio::test]
    async fn probe_403_counts_as_reachable() {
        let addr = spawn_status_server("403 Forbidden", ACCESS_DENIED).await;
        let registry = HealthRegistry::new();
        storage_at(format!("http://{addr}"))
            .await
            .check_health(&registry)
            .await;
        assert!(matches!(
            registry.snapshot().await.get("s3_storage").unwrap().status,
            Status::Healthy
        ));
        assert!(
            registry.is_ready().await,
            "a 403 bucket that boots must stay ready"
        );
    }

    /// A 403 that names the credentials will not fix itself: readiness must go red rather
    /// than report Healthy while every upload is being rejected.
    #[tokio::test]
    async fn probe_invalid_credentials_is_fatal() {
        let addr = spawn_status_server("403 Forbidden", INVALID_ACCESS_KEY).await;
        let registry = HealthRegistry::new();
        storage_at(format!("http://{addr}"))
            .await
            .check_health(&registry)
            .await;
        let snap = registry.snapshot().await.get("s3_storage").unwrap().clone();
        assert!(
            matches!(snap.status, Status::Deceased),
            "wrong credentials need an operator: {snap:?}"
        );
        assert!(
            !registry.is_ready().await,
            "a deployment with rejected credentials must not report ready"
        );
    }

    #[tokio::test]
    async fn probe_missing_bucket_is_fatal() {
        let addr = spawn_status_server("404 Not Found", "").await;
        let registry = HealthRegistry::new();
        storage_at(format!("http://{addr}"))
            .await
            .check_health(&registry)
            .await;
        assert!(matches!(
            registry.snapshot().await.get("s3_storage").unwrap().status,
            Status::Deceased
        ));
    }

    #[tokio::test]
    async fn probe_network_failure_is_transient() {
        let registry = HealthRegistry::new();
        storage_at("http://127.0.0.1:9".into())
            .await
            .check_health(&registry)
            .await;
        assert!(matches!(
            registry.snapshot().await.get("s3_storage").unwrap().status,
            Status::Sick
        ));
    }

    /// An endpoint that accepts TCP and then stops answering must not hang the probe:
    /// the 5s bound turns the hang into a `Sick` result.
    #[tokio::test]
    async fn probe_hung_endpoint_is_transient_not_hung() {
        let addr = common::spawn_hung_server().await;

        let registry = HealthRegistry::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            storage_at(format!("http://{addr}"))
                .await
                .check_health(&registry),
        )
        .await
        .expect("a wedged endpoint must cost one bounded probe, not hang the loop");
        assert!(matches!(
            registry.snapshot().await.get("s3_storage").unwrap().status,
            Status::Sick
        ));
    }
}
