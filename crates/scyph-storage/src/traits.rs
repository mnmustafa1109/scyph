//! Abstract interface definition for object storage providers.

use crate::error::StorageError;
use bytes::Bytes;

/// Asynchronous interface for cloud and local object storage providers.
///
/// `StorageService` defines the canonical async API for object storage operations used
/// throughout the scyph ecosystem. Concrete implementations include:
/// - [`S3StorageService`](crate::s3::S3StorageService) — AWS S3 / MinIO / Cloudflare R2 backend
/// - [`InMemoryStorageService`](crate::memory::InMemoryStorageService) — In-memory mock for tests
/// - Custom implementations for other backends (local filesystem, GCS, Azure Blob, etc.)
///
/// ## No `async_trait` Required
///
/// This trait uses **Return Position Impl Trait in Traits (RPITIT)**, stabilized in Rust 1.75.
/// There is no need for the `#[async_trait]` macro — simply `impl StorageService for YourType`
/// with `async fn` methods directly:
///
/// ```rust,ignore
/// use bytes::Bytes;
/// use scyph_storage::{StorageService, StorageError};
///
/// struct LocalFsStorage {
///     base_dir: std::path::PathBuf,
/// }
///
/// impl StorageService for LocalFsStorage {
///     async fn store(&self, path: &str, _content_type: &str, data: Bytes) -> Result<String, StorageError> {
///         let full = self.base_dir.join(path);
///         if let Some(parent) = full.parent() {
///             tokio::fs::create_dir_all(parent).await
///                 .map_err(|e| StorageError::Internal(e.to_string()))?;
///         }
///         tokio::fs::write(&full, &data).await
///             .map_err(|e| StorageError::Internal(e.to_string()))?;
///         Ok(path.to_string())
///     }
///
///     async fn retrieve(&self, file_url: &str) -> Result<Bytes, StorageError> {
///         let full = self.base_dir.join(file_url);
///         let data = tokio::fs::read(&full).await
///             .map_err(|_| StorageError::NotFound(file_url.to_string()))?;
///         Ok(Bytes::from(data))
///     }
///
///     async fn get_view_url(&self, file_url: &str, _expires_in_seconds: u64) -> Result<String, StorageError> {
///         Ok(format!("file://{}/{file_url}", self.base_dir.display()))
///     }
///
///     async fn get_view_urls(&self, file_urls: Vec<String>, expires: u64) -> Result<Vec<String>, StorageError> {
///         let mut results = Vec::with_capacity(file_urls.len());
///         for url in file_urls {
///             results.push(self.get_view_url(&url, expires).await?);
///         }
///         Ok(results)
///     }
///
///     async fn get_download_url(&self, file_url: &str, _download_name: &str, _expires: u64) -> Result<String, StorageError> {
///         Ok(format!("file://{}/{file_url}", self.base_dir.display()))
///     }
///
///     async fn delete(&self, file_url: &str) -> Result<(), StorageError> {
///         let full = self.base_dir.join(file_url);
///         tokio::fs::remove_file(&full).await
///             .map_err(|e| StorageError::Internal(e.to_string()))
///     }
///
///     async fn test_connection(&self) -> Result<(), StorageError> {
///         if self.base_dir.exists() { Ok(()) }
///         else { Err(StorageError::Configuration("Base directory does not exist".to_string())) }
///     }
/// }
/// ```
///
/// ## Using as a Trait Object
///
/// Because `StorageService` uses RPITIT, it cannot be used as a bare `dyn StorageService`
/// (the trait is not object-safe). Wrap it in `Arc<dyn StorageService>` using a
/// helper crate like `async-trait` or use a concrete type / enum dispatch instead:
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use scyph_storage::{InMemoryStorageService, StorageService};
///
/// // Concrete type works everywhere:
/// let storage: Arc<InMemoryStorageService> = Arc::new(InMemoryStorageService::new());
/// ```
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
