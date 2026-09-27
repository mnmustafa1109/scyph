//! Storage service extension trait for combined original and thumbnail upload.

use bytes::Bytes;

use super::config::{DEFAULT_THUMBNAIL_SUBFOLDER, ThumbnailConfig};
use crate::{error::StorageError, traits::StorageService};

/// Result of storing an original file alongside its derived thumbnail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailStoreResult {
    /// Destination path of the original stored file.
    pub original_key: String,
    /// Destination path of the generated thumbnail.
    pub thumbnail_key: String,
}

/// Extension trait for [`StorageService`] providing atomic-style original + thumbnail upload.
pub trait StorageThumbnailExt: StorageService {
    /// Stores the original file and generates + stores an image thumbnail in a single operation.
    ///
    /// The thumbnail storage key is derived using [`ThumbnailConfig::derive_key`] with
    /// [`DEFAULT_THUMBNAIL_SUBFOLDER`].
    ///
    /// # Arguments
    ///
    /// * `original_key` - Path where the original file will be stored.
    /// * `original_content_type` - MIME content-type of the original file.
    /// * `data` - Zero-copy byte buffer of the file.
    /// * `config` - Thumbnail configuration (dimensions, format, etc.).
    ///
    /// # Returns
    ///
    /// Returns [`ThumbnailStoreResult`] containing both the original and thumbnail storage keys.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::ValidationError`] if thumbnail generation fails,
    /// or [`StorageError`] if either upload fails.
    fn store_with_thumbnail(
        &self,
        original_key: &str,
        original_content_type: &str,
        data: Bytes,
        config: ThumbnailConfig,
    ) -> impl std::future::Future<Output = Result<ThumbnailStoreResult, StorageError>> + Send;
}

impl<T: StorageService + ?Sized> StorageThumbnailExt for T {
    async fn store_with_thumbnail(
        &self,
        original_key: &str,
        original_content_type: &str,
        data: Bytes,
        config: ThumbnailConfig,
    ) -> Result<ThumbnailStoreResult, StorageError> {
        let thumb_bytes = config.generate(&data)?;
        let thumb_key = config.derive_key(original_key, DEFAULT_THUMBNAIL_SUBFOLDER);
        let thumb_content_type = config.content_type();

        let stored_orig = self
            .store(original_key, original_content_type, data)
            .await?;
        let stored_thumb = self
            .store(&thumb_key, thumb_content_type, Bytes::from(thumb_bytes))
            .await?;

        Ok(ThumbnailStoreResult {
            original_key: stored_orig,
            thumbnail_key: stored_thumb,
        })
    }
}
