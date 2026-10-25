//! AWS S3 and S3-compatible (MinIO, LocalStack, Wasabi, Cloudflare R2) storage client.

mod config;
mod service;

pub use config::sanitize_endpoint;

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
    /// Resolves the storage key for a raw key or a (possibly presigned) file URL.
    ///
    /// Strips query parameters, the public URL prefix, and the `{bucket}/`
    /// path-style segment, so handlers can pass user-supplied URLs straight to
    /// [`delete`](crate::StorageService::delete) or other key-based operations.
    pub fn normalize_key<'a>(&'a self, file_url: &'a str) -> &'a str {
        // Strip query parameters if present (e.g. from previously signed URLs)
        let path = file_url.split('?').next().unwrap_or(file_url);

        // Strip public URL prefix if present
        let key = if let Some(stripped) = path.strip_prefix(&self.public_url_prefix) {
            stripped.strip_prefix('/').unwrap_or(stripped)
        } else {
            path
        };

        // Strip "{bucket}/" segment for path-style addressing. Must include the slash,
        // otherwise a bucket named "data" would corrupt a key like "database/x.png".
        let bucket_prefix = format!("{}/", self.bucket);
        let key = if let Some(stripped) = key.strip_prefix(&bucket_prefix) {
            stripped
        } else {
            key
        };

        key.strip_prefix('/').unwrap_or(key)
    }
}
