//! AWS S3 / S3-compatible client configuration and environment variable loaders.
//!
//! Provides [`S3StorageService::new`] (explicit credentials) and [`S3StorageService::from_env`]
//! (environment-driven) constructors for building a configured storage service.
//!
//! ## Environment Variables
//!
//! | Variable | Required | Default | Description |
//! |----------|----------|---------|-------------|
//! | `S3_BUCKET` | **Yes** | — | S3 bucket name |
//! | `S3_ENDPOINT_URL` | No | AWS default endpoint | Custom S3-compatible endpoint (MinIO, LocalStack) |
//! | `S3_PUBLIC_URL_PREFIX` | No | Derived from bucket/region | Browser-accessible presigned URL base |
//! | `S3_REGION` / `AWS_REGION` | No | `us-east-1` | AWS region |
//! | `AWS_ACCESS_KEY_ID` | No | IAM role / instance profile | Static access key |
//! | `AWS_SECRET_ACCESS_KEY` | No | IAM role / instance profile | Static secret key |
//!
//! ## Path-Style vs Virtual-Hosted Addressing
//!
//! When `S3_ENDPOINT_URL` is set to a custom non-AWS endpoint (e.g. `http://localhost:9000`),
//! path-style addressing is automatically enabled (`force_path_style(true)`). This is required
//! for MinIO, LocalStack, and Ceph RGW, which do not support virtual-hosted bucket addressing.
//!
//! AWS S3 uses virtual-hosted style by default:
//! ```text
//! https://{bucket}.s3.{region}.amazonaws.com/{key}
//! ```
//!
//! MinIO / LocalStack uses path-style:
//! ```text
//! http://localhost:9000/{bucket}/{key}
//! ```
//!
//! ## MinIO Local Development Example
//!
//! ```env
//! S3_BUCKET=my-bucket
//! S3_ENDPOINT_URL=http://localhost:9000
//! S3_PUBLIC_URL_PREFIX=http://localhost:9000
//! S3_REGION=us-east-1
//! AWS_ACCESS_KEY_ID=minioadmin
//! AWS_SECRET_ACCESS_KEY=minioadmin
//! ```
//!
//! ## AWS Production Example (IAM Role, no static keys)
//!
//! ```env
//! S3_BUCKET=my-prod-bucket
//! S3_REGION=us-east-1
//! S3_PUBLIC_URL_PREFIX=https://cdn.example.com
//! # AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY omitted — uses EC2 instance profile / ECS task role
//! ```

use std::time::Duration;

use crate::error::StorageError;
use crate::s3::S3StorageService;
use crate::traits::StorageService;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, config::Credentials};

/// Bounds every S3 request. The SDK defaults to a connect timeout only, so an endpoint
/// that accepts TCP and then stops answering (paused MinIO, broken proxy, LB misroute)
/// would hang health probes and uploads forever. `read_timeout` is per socket read, so
/// slow-but-flowing transfers still complete.
/// shortcut: no overall operation timeout, so an endpoint trickling bytes can still hold a request open; set `operation_timeout` once uploads are size-bounded.
fn request_timeouts() -> aws_sdk_s3::config::timeout::TimeoutConfig {
    aws_sdk_s3::config::timeout::TimeoutConfig::builder()
        .connect_timeout(Duration::from_secs(3))
        .read_timeout(Duration::from_secs(30))
        .build()
}

/// Sanitizes an endpoint URL by trimming trailing slashes and, for path-style endpoints (such as MinIO),
/// stripping any trailing bucket path (e.g. `/{bucket}`) to prevent duplicate bucket paths in generated URLs.
pub fn sanitize_endpoint(endpoint: &str, bucket: &str, is_path_style: bool) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if is_path_style {
        let bucket_suffix = format!("/{bucket}");
        if let Some(stripped) = trimmed.strip_suffix(&bucket_suffix) {
            return stripped.to_string();
        }
    }
    trimmed.to_string()
}

impl S3StorageService {
    /// Creates an [`S3StorageService`] instance with explicit credentials and endpoint configuration.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Target S3 bucket name.
    /// * `public_url_prefix` - Public browser-accessible URL prefix for generated presigned links.
    ///   For MinIO/LocalStack (path-style), this should be the server endpoint (e.g. `http://localhost:9000`),
    ///   without the bucket name appended. If a trailing `/{bucket}` is present, it will be automatically stripped.
    /// * `endpoint` - Backend S3 API endpoint URL (e.g. `http://localhost:9000` or `https://s3.us-east-1.amazonaws.com`).
    /// * `region` - AWS Region string (e.g. `"us-east-1"`).
    /// * `access_key` - AWS Access Key ID.
    /// * `secret_key` - AWS Secret Access Key.
    pub async fn new(
        bucket: String,
        public_url_prefix: String,
        endpoint: String,
        region: String,
        access_key: String,
        secret_key: String,
    ) -> Self {
        let creds = Credentials::new(access_key, secret_key, None, None, "static");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(Region::new(region))
            .timeout_config(request_timeouts())
            .load()
            .await;

        let is_custom_endpoint = !endpoint.contains("amazonaws.com");

        let clean_endpoint = sanitize_endpoint(&endpoint, &bucket, is_custom_endpoint);
        let clean_presign_endpoint =
            sanitize_endpoint(&public_url_prefix, &bucket, is_custom_endpoint);

        let mut s3_builder =
            aws_sdk_s3::config::Builder::from(&shared_config).endpoint_url(&clean_endpoint);
        if is_custom_endpoint {
            s3_builder = s3_builder.force_path_style(true);
        }
        let s3_config = s3_builder.build();

        let mut presign_builder =
            aws_sdk_s3::config::Builder::from(&shared_config).endpoint_url(&clean_presign_endpoint);
        if is_custom_endpoint {
            presign_builder = presign_builder.force_path_style(true);
        }
        let presign_config = presign_builder.build();

        Self {
            client: Client::from_conf(s3_config),
            presign_client: Client::from_conf(presign_config),
            bucket,
            public_url_prefix: public_url_prefix.trim_end_matches('/').to_string(),
        }
    }

    /// Constructs an [`S3StorageService`] instance from environment variables:
    ///
    /// - `S3_BUCKET` *(Required)*: Target S3 bucket name.
    /// - `S3_PUBLIC_URL_PREFIX` *(Optional)*: Browser-accessible public endpoint/CDN prefix
    ///   (e.g. `http://localhost:9000` for MinIO or `https://cdn.example.com`).
    /// - `S3_ENDPOINT_URL` *(Optional)*: Custom internal S3 endpoint for MinIO/LocalStack (e.g. `http://localhost:9000`).
    /// - `S3_REGION` / `AWS_REGION` *(Optional)*: AWS region, defaults to `"us-east-1"`.
    /// - `AWS_ACCESS_KEY_ID` *(Optional)*: Static AWS access key.
    /// - `AWS_SECRET_ACCESS_KEY` *(Optional)*: Static AWS secret key.
    ///
    /// When using custom endpoints (MinIO/LocalStack), path-style addressing (`force_path_style(true)`)
    /// is automatically enabled, and any trailing `/{bucket}` in `S3_PUBLIC_URL_PREFIX` is defensively
    /// stripped to prevent duplicate bucket path segments in presigned URLs.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Configuration`] if `S3_BUCKET` is not set, if the bucket
    /// does not answer within a 5s connection ping, or if the ping fails (dead endpoint,
    /// wrong or missing credentials, or a missing bucket) — so broken S3 configuration
    /// fails at boot instead of on the first upload.
    ///
    /// The boot ping uses `ListObjectsV2`, which requires the `s3:ListBucket` IAM
    /// permission. A `403 Forbidden` answer whose body says `AccessDenied` — a policy
    /// without `s3:ListBucket` — means the bucket is reachable, so it is logged as a
    /// warning and startup continues. A 403 whose body names bad credentials
    /// (`InvalidAccessKeyId`, `SignatureDoesNotMatch`, `ExpiredToken`,
    /// `InvalidClientTokenId`) is a configuration error and fails startup.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use scyph_storage::S3StorageService;
    ///
    /// async fn setup() {
    ///     let service = S3StorageService::from_env().await.expect("S3 initialized");
    /// }
    /// ```
    pub async fn from_env() -> Result<Self, StorageError> {
        let bucket = std::env::var("S3_BUCKET").map_err(|_| {
            StorageError::Configuration("S3_BUCKET environment variable must be set".into())
        })?;

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").ok();
        let public_url_prefix = std::env::var("S3_PUBLIC_URL_PREFIX").ok();
        let region_str = std::env::var("S3_REGION")
            .or_else(|_| std::env::var("AWS_REGION"))
            .unwrap_or_else(|_| "us-east-1".to_string());

        let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

        tracing::info!(
            bucket = %bucket,
            region = %region_str,
            endpoint = ?endpoint_url,
            public_prefix = ?public_url_prefix,
            "Initializing S3 storage service from environment"
        );

        let service = if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            let endpoint = endpoint_url
                .clone()
                .unwrap_or_else(|| format!("https://s3.{region_str}.amazonaws.com"));
            let is_custom = endpoint_url.is_some() || !endpoint.contains("amazonaws.com");
            let pub_prefix = public_url_prefix.unwrap_or_else(|| {
                if is_custom {
                    endpoint.clone()
                } else {
                    format!("https://{bucket}.s3.{region_str}.amazonaws.com")
                }
            });
            Self::new(bucket, pub_prefix, endpoint, region_str, ak, sk).await
        } else {
            let mut loader = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region_str.clone()))
                .timeout_config(request_timeouts());
            if let Some(ref url) = endpoint_url {
                loader = loader.endpoint_url(url);
            }
            let cfg = loader.load().await;

            let is_custom_endpoint = endpoint_url
                .as_ref()
                .map(|url| !url.contains("amazonaws.com"))
                .unwrap_or(false);

            let clean_endpoint = endpoint_url
                .as_ref()
                .map(|url| sanitize_endpoint(url, &bucket, is_custom_endpoint));

            let mut s3_builder = aws_sdk_s3::config::Builder::from(&cfg);
            if let Some(ref url) = clean_endpoint {
                s3_builder = s3_builder.endpoint_url(url);
            }
            if is_custom_endpoint {
                s3_builder = s3_builder.force_path_style(true);
            }
            let s3_config = s3_builder.build();

            let client = Client::from_conf(s3_config);

            let presign_client = if let Some(ref pub_url) = public_url_prefix {
                let clean_pub_url = sanitize_endpoint(pub_url, &bucket, is_custom_endpoint);
                let mut presign_builder =
                    aws_sdk_s3::config::Builder::from(&cfg).endpoint_url(clean_pub_url);
                if is_custom_endpoint {
                    presign_builder = presign_builder.force_path_style(true);
                }
                Client::from_conf(presign_builder.build())
            } else if let Some(ref ep) = clean_endpoint {
                let mut presign_builder = aws_sdk_s3::config::Builder::from(&cfg).endpoint_url(ep);
                if is_custom_endpoint {
                    presign_builder = presign_builder.force_path_style(true);
                }
                Client::from_conf(presign_builder.build())
            } else {
                client.clone()
            };

            let pub_prefix = public_url_prefix.unwrap_or_else(|| {
                if let Some(ref ep) = clean_endpoint {
                    ep.clone()
                } else {
                    format!("https://{bucket}.s3.{region_str}.amazonaws.com")
                }
            });

            Self {
                client,
                presign_client,
                bucket,
                public_url_prefix: pub_prefix.trim_end_matches('/').to_string(),
            }
        };

        // Fail fast at boot: a dead endpoint or bad credentials would otherwise
        // surface only on the first upload (mirrors notify's from_env handshake).
        // A 403 that only means "no s3:ListBucket" is not a failure: least-privilege
        // policies must not block boot.
        let endpoint_desc = endpoint_url.as_deref().unwrap_or("default AWS endpoint");
        match tokio::time::timeout(Duration::from_secs(5), service.test_connection()).await {
            Ok(Ok(())) => {}
            Ok(Err(StorageError::Forbidden(_))) => {
                tracing::warn!(
                    bucket = %service.bucket,
                    "S3 boot check returned 403 Forbidden: the bucket is reachable but the \
                     credentials may lack the s3:ListBucket permission; continuing without \
                     a full boot check"
                );
            }
            Ok(Err(StorageError::Configuration(detail))) => {
                // `test_connection` only reports Configuration for credentials S3 rejected.
                return Err(StorageError::Configuration(format!(
                    "{detail} - check the credentials for {endpoint_desc} (AWS_ACCESS_KEY_ID, \
                     AWS_SECRET_ACCESS_KEY)"
                )));
            }
            Ok(Err(e)) => {
                return Err(StorageError::Configuration(format!(
                    "S3 boot check against {endpoint_desc} failed: {e} - check S3_BUCKET, \
                     S3_ENDPOINT_URL, AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY"
                )));
            }
            Err(_) => {
                return Err(StorageError::Configuration(format!(
                    "S3 connection to {endpoint_desc} timed out after 5s"
                )));
            }
        }
        Ok(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SDK ships with a connect timeout only, so both constructors must wire in a
    /// read timeout — otherwise an endpoint that accepts TCP and stops answering hangs
    /// health probes and uploads forever.
    #[tokio::test]
    async fn new_wires_request_timeouts() {
        let service = tokio::time::timeout(
            Duration::from_secs(10),
            S3StorageService::new(
                "my-bucket".into(),
                "http://localhost:9000".into(),
                "http://localhost:9000".into(),
                "us-east-1".into(),
                "test-key".into(),
                "test-secret".into(),
            ),
        )
        .await
        .expect("constructing the client must not hang");

        let timeouts = service
            .client
            .config()
            .timeout_config()
            .expect("timeout config must be wired into the S3 client");
        assert_eq!(timeouts.connect_timeout(), Some(Duration::from_secs(3)));
        assert_eq!(timeouts.read_timeout(), Some(Duration::from_secs(30)));
    }
}
