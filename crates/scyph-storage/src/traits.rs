//! Abstract interface definition for object storage providers.

use crate::error::StorageError;
use bytes::Bytes;

/// Asynchronous interface for cloud and local object storage providers.
///
/// Implemented by concrete storage backends like [`S3StorageService`](crate::s3::S3StorageService)
/// as well as custom in-memory or mock adapters for automated integration tests.
pub trait StorageService: Send + Sync + 'static {
    /// Uploads raw binary data to the target storage path with the given MIME content-type.
    ///
    /// # Arguments
    ///
    /// * `path` - Destination key/path in the bucket (e.g., `"avatars/018f2d5e-4a6c-7000-8000-000000000001.png"`).
    /// * `content_type` - HTTP Content-Type header (e.g., `"image/png"`, `"application/pdf"`).
    /// * `data` - Zero-copy [`Bytes`] buffer holding the file payload.
    ///
    /// # Returns
    ///
    /// The stored relative path or identifier on success.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::S3`] if upload transmission or authentication fails.
    fn store(
        &self,
        path: &str,
        content_type: &str,
        data: Bytes,
    ) -> impl Future<Output = Result<String, StorageError>> + Send;

    /// Retrieves an object from storage by its path or public URL.
    ///
    /// # Arguments
    ///
    /// * `file_url` - The stored path or full URL of the target object.
    ///
    /// # Returns
    ///
    /// The downloaded payload as an owned [`Bytes`] buffer.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`] if the object does not exist, or [`StorageError::S3`] on retrieval failure.
    fn retrieve(&self, file_url: &str) -> impl Future<Output = Result<Bytes, StorageError>> + Send;

    /// Generates a temporary, cryptographically signed URL allowing client browsers to view/stream an object inline.
    ///
    /// # Arguments
    ///
    /// * `file_url` - The stored path or URL identifier of the object.
    /// * `expires_in_seconds` - Validity duration for the signed link in seconds (e.g. `3600` for 1 hour).
    ///
    /// # Returns
    ///
    /// A pre-signed HTTPS URL containing authentication signature query parameters.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Presign`] if signature calculation or configuration fails.
    fn get_view_url(
        &self,
        file_url: &str,
        expires_in_seconds: u64,
    ) -> impl Future<Output = Result<String, StorageError>> + Send;

    /// Generates temporary view URLs for a collection of objects.
    ///
    /// # Arguments
    ///
    /// * `file_urls` - List of object path identifiers.
    /// * `expires_in_seconds` - Validity duration in seconds.
    ///
    /// # Returns
    ///
    /// An ordered vector of signed view URLs corresponding to the input list.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Presign`] if signature calculation fails for any URL.
    fn get_view_urls(
        &self,
        file_urls: Vec<String>,
        expires_in_seconds: u64,
    ) -> impl Future<Output = Result<Vec<String>, StorageError>> + Send;

    /// Generates a signed download URL that forces browsers to trigger a file save dialog with the given filename.
    ///
    /// Sets the HTTP `ResponseContentDisposition` header to `attachment; filename="<download_name>"`.
    ///
    /// # Arguments
    ///
    /// * `file_url` - The stored path or URL of the target file.
    /// * `download_name` - The default filename presented to the user's browser.
    /// * `expires_in_seconds` - Validity duration in seconds.
    ///
    /// # Returns
    ///
    /// A pre-signed download URL.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Presign`] if signature generation fails.
    fn get_download_url(
        &self,
        file_url: &str,
        download_name: &str,
        expires_in_seconds: u64,
    ) -> impl Future<Output = Result<String, StorageError>> + Send;

    /// Permanently deletes an object from storage.
    ///
    /// # Arguments
    ///
    /// * `file_url` - The stored path or URL identifier of the object to delete.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::S3`] if deletion fails.
    fn delete(&self, file_url: &str) -> impl Future<Output = Result<(), StorageError>> + Send;

    /// Performs an active probe against the storage bucket to verify authentication credentials and connectivity.
    ///
    /// Used by startup checks and background health monitoring tasks.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::S3`] or [`StorageError::Configuration`] if the bucket cannot be reached.
    fn test_connection(&self) -> impl Future<Output = Result<(), StorageError>> + Send;
}
