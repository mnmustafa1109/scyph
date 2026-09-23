//! AWS S3 and S3-compatible (MinIO, LocalStack, Wasabi, Cloudflare R2) storage client.

use crate::{error::StorageError, traits::StorageService};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    config::Credentials,
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use bytes::Bytes;
use std::time::Duration;

/// Production-grade S3 storage client implementing [`StorageService`].
///
/// Features a **dual-client architecture**:
/// - An internal client (`client`) configured for internal backend S3 API calls (e.g., `http://minio:9000`).
/// - A presigning client (`presign_client`) configured with the browser-accessible public endpoint/CDN URL
///   (e.g., `https://cdn.myapp.com` or `http://localhost:9000/my-bucket`).
pub struct S3StorageService {
    client: Client,
    presign_client: Client,
    bucket: String,
    public_url_prefix: String,
}

impl S3StorageService {
    /// Creates an [`S3StorageService`] instance with explicit credentials and endpoint configuration.
    ///
    /// # Arguments
    ///
    /// * `bucket` - Target S3 bucket name.
    /// * `public_url_prefix` - Public browser-accessible URL prefix for generated presigned links.
    /// * `endpoint` - Backend S3 API endpoint URL.
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
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .endpoint_url(&endpoint)
            .force_path_style(true)
            .build();

        let presign_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .endpoint_url(&public_url_prefix)
            .force_path_style(true)
            .build();

        Self {
            client: Client::from_conf(s3_config),
            presign_client: Client::from_conf(presign_config),
            bucket,
            public_url_prefix,
        }
    }

    /// Constructs an [`S3StorageService`] instance from environment variables:
    ///
    /// - `S3_BUCKET` *(Required)*: Target S3 bucket name.
    /// - `S3_PUBLIC_URL_PREFIX` *(Optional)*: Browser-accessible public endpoint/CDN prefix.
    /// - `S3_ENDPOINT_URL` *(Optional)*: Custom internal S3 endpoint for MinIO/LocalStack.
    /// - `S3_REGION` / `AWS_REGION` *(Optional)*: AWS region, defaults to `"us-east-1"`.
    /// - `AWS_ACCESS_KEY_ID` *(Optional)*: Static AWS access key.
    /// - `AWS_SECRET_ACCESS_KEY` *(Optional)*: Static AWS secret key.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Configuration`] if `S3_BUCKET` is not set.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use scyph_storage::S3StorageService;
    ///
    /// async fn setup() {
    ///     std::env::set_var("S3_BUCKET", "my-app-uploads");
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

        if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            let endpoint = endpoint_url
                .unwrap_or_else(|| format!("https://s3.{region_str}.amazonaws.com"));
            let pub_prefix = public_url_prefix.unwrap_or_else(|| format!("{endpoint}/{bucket}"));
            Ok(Self::new(bucket, pub_prefix, endpoint, region_str, ak, sk).await)
        } else {
            let mut loader = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region_str));
            if let Some(ref url) = endpoint_url {
                loader = loader.endpoint_url(url);
            }
            let cfg = loader.load().await;

            let s3_builder = aws_sdk_s3::config::Builder::from(&cfg).force_path_style(true);
            let s3_config = if let Some(ref url) = endpoint_url {
                s3_builder.endpoint_url(url).build()
            } else {
                s3_builder.build()
            };

            let client = Client::from_conf(s3_config);

            let presign_client = if let Some(ref pub_url) = public_url_prefix {
                let presign_config = aws_sdk_s3::config::Builder::from(&cfg)
                    .endpoint_url(pub_url)
                    .force_path_style(true)
                    .build();
                Client::from_conf(presign_config)
            } else {
                client.clone()
            };

            let pub_prefix = public_url_prefix.unwrap_or_else(|| bucket.clone());

            Ok(Self {
                client,
                presign_client,
                bucket,
                public_url_prefix: pub_prefix,
            })
        }
    }

    fn normalize_key<'a>(&'a self, file_url: &'a str) -> &'a str {
        let prefix_slash = format!("{}/", self.public_url_prefix);
        if let Some(stripped) = file_url.strip_prefix(&prefix_slash) {
            stripped
        } else {
            file_url
        }
    }
}

impl StorageService for S3StorageService {
    async fn store(
        &self,
        path: &str,
        content_type: &str,
        data: Bytes,
    ) -> Result<String, StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(path)
            .content_type(content_type)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| StorageError::S3(format!("S3 upload error: {path}: {e}")))?;

        Ok(path.to_string())
    }

    async fn retrieve(&self, file_url: &str) -> Result<Bytes, StorageError> {
        let key = self.normalize_key(file_url);

        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3(format!("S3 read error: {key}: {e}")))?;

        let data = object
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3(format!("S3 body read error: {key}: {e}")))?;

        Ok(data.into_bytes())
    }

    async fn get_view_url(
        &self,
        file_url: &str,
        expires_in_seconds: u64,
    ) -> Result<String, StorageError> {
        let key = self.normalize_key(file_url);
        let expires_in = Duration::from_secs(expires_in_seconds);
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Presign(format!("Presigning config error: {e}")))?;

        let presigned_request = self
            .presign_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::Presign(format!("S3 presign view error: {key}: {e}")))?;

        Ok(presigned_request.uri().to_string())
    }

    async fn get_view_urls(
        &self,
        file_urls: Vec<String>,
        expires_in_seconds: u64,
    ) -> Result<Vec<String>, StorageError> {
        let mut results = Vec::with_capacity(file_urls.len());
        for url in file_urls {
            results.push(self.get_view_url(&url, expires_in_seconds).await?);
        }
        Ok(results)
    }

    async fn get_download_url(
        &self,
        file_url: &str,
        download_name: &str,
        expires_in_seconds: u64,
    ) -> Result<String, StorageError> {
        let key = self.normalize_key(file_url);
        let expires_in = Duration::from_secs(expires_in_seconds);
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Presign(format!("Presigning config error: {e}")))?;

        let presigned_request = self
            .presign_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_disposition(format!("attachment; filename=\"{download_name}\""))
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::Presign(format!("S3 presign download error: {key}: {e}")))?;

        Ok(presigned_request.uri().to_string())
    }

    async fn delete(&self, file_url: &str) -> Result<(), StorageError> {
        let key = self.normalize_key(file_url);
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3(format!("S3 delete error: {key}: {e}")))?;
        Ok(())
    }

    async fn test_connection(&self) -> Result<(), StorageError> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(|e| {
                StorageError::S3(format!("Failed to connect to storage bucket '{}': {e}", self.bucket))
            })?;

        Ok(())
    }
}
