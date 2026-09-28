//! AWS S3 and S3-compatible (MinIO, LocalStack, Wasabi, Cloudflare R2) storage client.

mod config;
mod service;

use aws_sdk_s3::Client;

/// Production-grade S3 storage client implementing [`StorageService`](crate::StorageService).
///
/// Features a **dual-client architecture**:
/// - An internal client (`client`) configured for internal backend S3 API calls (e.g., `http://minio:9000`).
/// - A presigning client (`presign_client`) configured with the browser-accessible public endpoint/CDN URL
///   (e.g., `https://cdn.myapp.com` or `http://localhost:9000`).
///
/// For path-style storage providers like MinIO or LocalStack, the public URL prefix should point to
/// the server endpoint without appending the bucket name (e.g., `http://localhost:9000`), as the AWS SDK
/// automatically appends `/{bucket}/{key}` when path-style addressing is enabled.
pub struct S3StorageService {
    pub(crate) client: Client,
    pub(crate) presign_client: Client,
    pub(crate) bucket: String,
    pub(crate) public_url_prefix: String,
}

impl S3StorageService {
    pub(crate) fn normalize_key<'a>(&'a self, file_url: &'a str) -> &'a str {
        // Strip query parameters if present (e.g. from previously signed URLs)
        let path = file_url.split('?').next().unwrap_or(file_url);

        // Strip public URL prefix if present
        let key = if let Some(stripped) = path.strip_prefix(&self.public_url_prefix) {
            stripped.strip_prefix('/').unwrap_or(stripped)
        } else {
            path
        };

        // Strip bucket name if path-style addressing prefix is included
        let key = if let Some(stripped) = key.strip_prefix(&self.bucket) {
            stripped.strip_prefix('/').unwrap_or(stripped)
        } else {
            key
        };

        key.strip_prefix('/').unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_key_variations() {
        let dummy_client = Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
                .build(),
        );
        let service = S3StorageService {
            client: dummy_client.clone(),
            presign_client: dummy_client,
            bucket: "my-bucket".to_string(),
            public_url_prefix: "http://localhost:9000".to_string(),
        };

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
            service.normalize_key(
                "http://localhost:9000/my-bucket/uploads/photo.png?X-Amz-Signature=123"
            ),
            "uploads/photo.png"
        );
    }
}
