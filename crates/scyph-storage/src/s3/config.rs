//! S3 client configuration and environment loaders.

use crate::error::StorageError;
use crate::s3::S3StorageService;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, config::Credentials};

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

        let is_custom_endpoint = !endpoint.contains("amazonaws.com");

        let mut s3_builder = aws_sdk_s3::config::Builder::from(&shared_config)
            .endpoint_url(&endpoint);
        if is_custom_endpoint {
            s3_builder = s3_builder.force_path_style(true);
        }
        let s3_config = s3_builder.build();

        let mut presign_builder = aws_sdk_s3::config::Builder::from(&shared_config)
            .endpoint_url(&public_url_prefix);
        if is_custom_endpoint {
            presign_builder = presign_builder.force_path_style(true);
        }
        let presign_config = presign_builder.build();

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
                .clone()
                .unwrap_or_else(|| format!("https://s3.{region_str}.amazonaws.com"));
            let pub_prefix = public_url_prefix.unwrap_or_else(|| {
                if endpoint_url.is_some() {
                    format!("{endpoint}/{bucket}")
                } else {
                    format!("https://{bucket}.s3.{region_str}.amazonaws.com")
                }
            });
            Ok(Self::new(bucket, pub_prefix, endpoint, region_str, ak, sk).await)
        } else {
            let mut loader =
                aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region_str));
            if let Some(ref url) = endpoint_url {
                loader = loader.endpoint_url(url);
            }
            let cfg = loader.load().await;

            let s3_builder = aws_sdk_s3::config::Builder::from(&cfg);
            let s3_config = if let Some(ref url) = endpoint_url {
                s3_builder.endpoint_url(url).force_path_style(true).build()
            } else {
                s3_builder.build()
            };

            let client = Client::from_conf(s3_config);

            let presign_client = if let Some(ref pub_url) = public_url_prefix {
                let mut presign_builder =
                    aws_sdk_s3::config::Builder::from(&cfg).endpoint_url(pub_url);
                if endpoint_url.is_some() {
                    presign_builder = presign_builder.force_path_style(true);
                }
                Client::from_conf(presign_builder.build())
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
}
