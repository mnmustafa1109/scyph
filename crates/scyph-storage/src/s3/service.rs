//! Implementation of `StorageService` trait for `S3StorageService`.

use crate::{error::StorageError, s3::S3StorageService, traits::StorageService};
use aws_sdk_s3::{presigning::PresigningConfig, primitives::ByteStream};
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
