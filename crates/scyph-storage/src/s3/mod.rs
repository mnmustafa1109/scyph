//! AWS S3 and S3-compatible (MinIO, LocalStack, Wasabi, Cloudflare R2) storage client.

mod config;
mod service;

use aws_sdk_s3::Client;

/// Production-grade S3 storage client implementing [`StorageService`](crate::StorageService).
///
/// Features a **dual-client architecture**:
/// - An internal client (`client`) configured for internal backend S3 API calls (e.g., `http://minio:9000`).
/// - A presigning client (`presign_client`) configured with the browser-accessible public endpoint/CDN URL
///   (e.g., `https://cdn.myapp.com` or `http://localhost:9000/my-bucket`).
pub struct S3StorageService {
    pub(crate) client: Client,
    pub(crate) presign_client: Client,
    pub(crate) bucket: String,
    pub(crate) public_url_prefix: String,
}

impl S3StorageService {
    pub(crate) fn normalize_key<'a>(&'a self, file_url: &'a str) -> &'a str {
        if let Some(stripped) = file_url.strip_prefix(&self.public_url_prefix) {
            stripped.strip_prefix('/').unwrap_or(stripped)
        } else {
            file_url
        }
    }
}
