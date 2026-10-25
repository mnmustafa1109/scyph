//! [`StorageService`] implementation for AWS S3 and S3-compatible object storage.
//!
//! This module implements all methods of the [`StorageService`] trait for [`S3StorageService`],
//! including object upload (`store`), download (`retrieve`), signed view URLs, signed download
//! URLs (with `Content-Disposition: attachment`), object deletion, and bucket connectivity probes.
//!
//! ## Dual Client Architecture
//!
//! `S3StorageService` maintains **two separate AWS SDK clients**:
//!
//! - `client` — Internal client using the cluster-internal endpoint (e.g., `http://minio:9000`).
//!   Used for `PUT`, `GET`, and `DELETE` operations that run server-side.
//! - `presign_client` — Presign client using the public endpoint (e.g., `https://cdn.example.com`
//!   or `http://localhost:9000`). Used for generating presigned URLs that will be clicked by
//!   users' browsers. If no public prefix is configured separately, falls back to `client`.
//!
//! This separation means internal service traffic uses the fast private network while presigned
//! URLs point to the public-facing CDN or external endpoint.
//!
//! ## Key Normalization
//!
//! [`normalize_key`](S3StorageService::normalize_key) strips any `public_url_prefix` or full
//! URL prefix from incoming paths, allowing handlers to pass either a relative key
//! (`"avatars/abc.png"`) or a full presigned URL without special casing.
//!
//! ## Presigned URL Format
//!
//! All presigned URLs include standard AWS Signature v4 query parameters:
//! `X-Amz-Algorithm`, `X-Amz-Credential`, `X-Amz-Date`, `X-Amz-Expires`,
//! `X-Amz-Security-Token` (if using STS), and `X-Amz-Signature`.

use crate::{error::StorageError, s3::S3StorageService, traits::StorageService};
use aws_sdk_s3::{
    error::ProvideErrorMetadata, presigning::PresigningConfig, primitives::ByteStream,
};
use bytes::Bytes;
use std::time::Duration;

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
        let sanitized_name = download_name.replace(['"', '\r', '\n', '\\'], "");
        let expires_in = Duration::from_secs(expires_in_seconds);
        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Presign(format!("Presigning config error: {e}")))?;

        let presigned_request = self
            .presign_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_disposition(format!("attachment; filename=\"{sanitized_name}\""))
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
        // `HeadBucket` is a HEAD request, so its error body is empty: a 403 carries no
        // error code and "wrong credentials" and "policy without s3:ListBucket" are
        // indistinguishable. `ListObjectsV2` is a GET (same `s3:ListBucket` permission,
        // same LIST billing class) and answers with an XML `<Code>`, so probe with
        // `max-keys=1` to keep the listing to a single key.
        self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(1)
            .send()
            .await
            .map_err(|e| {
                let status = e.raw_response().map(|r| r.status().as_u16());
                let code = e.as_service_error().and_then(|err| err.code());
                // The SDK's `Display` is a flat "service error"; the error code is the
                // reliable detail, so it becomes a typed error rather than a suffix
                // callers have to string-match.
                let msg = match code {
                    Some(code) => {
                        format!(
                            "Failed to connect to storage bucket '{}': {code}",
                            self.bucket
                        )
                    }
                    None => format!("Failed to connect to storage bucket '{}': {e}", self.bucket),
                };
                match (status, code) {
                    // Wrong or expired credentials answer 403 too, but the body names
                    // them — that is bad configuration, not a reachable bucket.
                    (
                        _,
                        Some(
                            "InvalidAccessKeyId"
                            | "SignatureDoesNotMatch"
                            | "ExpiredToken"
                            | "InvalidClientTokenId",
                        ),
                    ) => StorageError::Configuration(msg),
                    // Missing bucket: `NoSuchBucket` in the body, or a bare 404 from a
                    // server that does not bother to send one.
                    (Some(404), _) | (_, Some("NoSuchBucket")) => StorageError::NotFound(msg),
                    // Reachable but not permitted: a least-privilege policy without
                    // `s3:ListBucket` (`AccessDenied`), or a gateway that answers 403
                    // without a parseable body. Both mean "the bucket is there".
                    (Some(403), _) | (_, Some("AccessDenied")) => StorageError::Forbidden(msg),
                    _ => StorageError::S3(msg),
                }
            })?;

        Ok(())
    }
}
