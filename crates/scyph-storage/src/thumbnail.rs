//! Image thumbnail generation utilities for storage pipelines.
//!
//! Provides configurable image decoding, aspect-ratio-preserving resizing,
//! format encoding, and storage path key derivation for thumbnail assets.

use std::future::Future;
use std::io::Cursor;
use std::path::Path;

use bytes::Bytes;
use image::{ImageFormat, ImageReader};

use crate::{error::StorageError, traits::StorageService};

/// Default square bounding dimension in pixels for thumbnails.
pub const DEFAULT_THUMBNAIL_DIMENSION: u32 = 200;

/// Default subdirectory name used when deriving thumbnail storage keys.
pub const DEFAULT_THUMBNAIL_SUBFOLDER: &str = "thumbnails";

/// Configuration builder for image thumbnail generation.
///
/// Encapsulates maximum bounding box constraints and the target output image encoding format.
///
/// # Examples
///
/// ```rust
/// use image::ImageFormat;
/// use scyph_storage::thumbnail::ThumbnailConfig;
///
/// let config = ThumbnailConfig::new(300, 300)
///     .with_format(ImageFormat::Png);
///
/// assert_eq!(config.max_width, 300);
/// assert_eq!(config.max_height, 300);
/// assert_eq!(config.format, ImageFormat::Png);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailConfig {
    /// Maximum allowable width in pixels.
    pub max_width: u32,
    /// Maximum allowable height in pixels.
    pub max_height: u32,
    /// Target image encoding format for the generated thumbnail (default: JPEG).
    pub format: ImageFormat,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            max_width: DEFAULT_THUMBNAIL_DIMENSION,
            max_height: DEFAULT_THUMBNAIL_DIMENSION,
            format: ImageFormat::Jpeg,
        }
    }
}

impl ThumbnailConfig {
    /// Creates a new [`ThumbnailConfig`] with custom maximum width and height bounds.
    ///
    /// By default, encodes output as JPEG.
    ///
    /// # Arguments
    ///
    /// * `max_width` - Maximum thumbnail width in pixels.
    /// * `max_height` - Maximum thumbnail height in pixels.
    pub const fn new(max_width: u32, max_height: u32) -> Self {
        Self {
            max_width,
            max_height,
            format: ImageFormat::Jpeg,
        }
    }

    /// Configures the destination image encoding format (e.g. JPEG, PNG, WebP).
    ///
    /// # Arguments
    ///
    /// * `format` - Target [`ImageFormat`].
    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    /// Generates a thumbnail buffer from the provided raw image bytes using this configuration.
    ///
    /// Preserves original aspect ratio while scaling down to fit within `(max_width, max_height)`.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes of the source image.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::ValidationError`] if image format is unsupported or bytes are corrupt,
    /// or [`StorageError::Internal`] if re-encoding fails.
    pub fn generate(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        generate_thumbnail_with_format(data, self.max_width, self.max_height, self.format)
    }

    /// Returns the canonical MIME content-type string for this configuration's target output format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use image::ImageFormat;
    /// use scyph_storage::thumbnail::ThumbnailConfig;
    ///
    /// let config = ThumbnailConfig::default().with_format(ImageFormat::WebP);
    /// assert_eq!(config.content_type(), "image/webp");
    /// ```
    pub const fn content_type(&self) -> &'static str {
        format_content_type(self.format)
    }

    /// Returns the canonical file extension (without leading dot) for this configuration's format.
    pub const fn extension(&self) -> &'static str {
        format_extension(self.format)
    }

    /// Derives the destination storage key for a thumbnail based on this configuration's format.
    ///
    /// Replaces the original extension with the target format extension and places the file in `subfolder`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use image::ImageFormat;
    /// use scyph_storage::thumbnail::ThumbnailConfig;
    ///
    /// let config = ThumbnailConfig::default().with_format(ImageFormat::WebP);
    /// let key = config.derive_key("avatars/user.png", "thumbnails");
    /// assert_eq!(key, "avatars/thumbnails/user.webp");
    /// ```
    pub fn derive_key(&self, original_key: &str, subfolder: &str) -> String {
        derive_thumbnail_key_with_format(original_key, subfolder, self.format)
    }
}

/// Generates a JPEG thumbnail buffer from raw image bytes.
///
/// Automatically detects the incoming image format (JPEG, PNG, WebP, GIF),
/// scales the image down preserving aspect ratio, and encodes the output as JPEG.
///
/// # Arguments
///
/// * `data` - Raw bytes of the source image.
/// * `max_width` - Maximum width boundary in pixels.
/// * `max_height` - Maximum height boundary in pixels.
///
/// # Returns
///
/// Returns a vector of bytes representing the encoded JPEG thumbnail.
///
/// # Errors
///
/// Returns [`StorageError::ValidationError`] if image format cannot be recognized or decoded,
/// or [`StorageError::Internal`] if thumbnail encoding fails.
pub fn generate_thumbnail(
    data: &[u8],
    max_width: u32,
    max_height: u32,
) -> Result<Vec<u8>, StorageError> {
    generate_thumbnail_with_format(data, max_width, max_height, ImageFormat::Jpeg)
}

/// Generates a thumbnail buffer encoded with a specified destination format.
///
/// # Arguments
///
/// * `data` - Raw bytes of the source image.
/// * `max_width` - Maximum width boundary in pixels.
/// * `max_height` - Maximum height boundary in pixels.
/// * `format` - Target [`ImageFormat`] (e.g. `ImageFormat::Jpeg`, `ImageFormat::Png`, `ImageFormat::WebP`).
///
/// # Returns
///
/// Returns a vector of bytes representing the encoded thumbnail image.
///
/// # Errors
///
/// Returns [`StorageError::ValidationError`] if source bytes cannot be read as an image,
/// or [`StorageError::Internal`] if thumbnail re-encoding fails.
pub fn generate_thumbnail_with_format(
    data: &[u8],
    max_width: u32,
    max_height: u32,
    format: ImageFormat,
) -> Result<Vec<u8>, StorageError> {
    let bound_w = max_width.max(1);
    let bound_h = max_height.max(1);

    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| {
            StorageError::validation(format!("Failed to inspect image format: {e}"), vec![])
        })?;

    reader.limits(image::Limits::default());

    let dynamic_img = reader
        .decode()
        .map_err(|e| StorageError::validation(format!("Failed to decode image: {e}"), vec![]))?;

    let thumbnail = dynamic_img.thumbnail(bound_w, bound_h);

    let mut output_buf = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut output_buf), format)
        .map_err(|e| StorageError::Internal(format!("Failed to encode thumbnail image: {e}")))?;

    Ok(output_buf)
}

/// Replaces a folder segment in a storage key string with a thumbnail subfolder.
///
/// If `folder_from` is found in `original_key`, it is replaced with `folder_to`.
/// Otherwise, falls back to [`derive_thumbnail_key`] with [`DEFAULT_THUMBNAIL_SUBFOLDER`].
///
/// # Examples
///
/// ```rust
/// use scyph_storage::thumbnail::thumbnail_key;
///
/// let original = "messages/images/photo_01.jpg";
/// let thumb = thumbnail_key(original, "messages/images/", "messages/thumbnails/");
/// assert_eq!(thumb, "messages/thumbnails/photo_01.jpg");
/// ```
pub fn thumbnail_key(original_key: &str, folder_from: &str, folder_to: &str) -> String {
    if original_key.contains(folder_from) {
        original_key.replacen(folder_from, folder_to, 1)
    } else {
        derive_thumbnail_key(original_key, DEFAULT_THUMBNAIL_SUBFOLDER)
    }
}

/// Derives a thumbnail storage key by inserting a subdirectory before the file name.
///
/// Normalizes any backslashes to forward slashes for cross-platform cloud storage consistency.
///
/// # Examples
///
/// ```rust
/// use scyph_storage::thumbnail::derive_thumbnail_key;
///
/// let original = "uploads/users/avatar.jpg";
/// let thumb = derive_thumbnail_key(original, "thumbnails");
/// assert_eq!(thumb, "uploads/users/thumbnails/avatar.jpg");
///
/// let root_file = "photo.jpg";
/// let thumb_root = derive_thumbnail_key(root_file, "thumbnails");
/// assert_eq!(thumb_root, "thumbnails/photo.jpg");
/// ```
pub fn derive_thumbnail_key(original_key: &str, subfolder: &str) -> String {
    let normalized = original_key.replace('\\', "/");
    let path = Path::new(&normalized);
    if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
        if parent.as_os_str().is_empty() {
            format!("{}/{}", subfolder, file_name.to_string_lossy())
        } else {
            format!(
                "{}/{}/{}",
                parent.to_string_lossy(),
                subfolder,
                file_name.to_string_lossy()
            )
        }
    } else {
        format!("{}/{}", subfolder, normalized)
    }
}

/// Returns the canonical MIME content-type string corresponding to an [`ImageFormat`].
pub const fn format_content_type(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Gif => "image/gif",
        ImageFormat::Bmp => "image/bmp",
        ImageFormat::Ico => "image/x-icon",
        ImageFormat::Tiff => "image/tiff",
        _ => "application/octet-stream",
    }
}

/// Returns the primary file extension (without leading dot) corresponding to an [`ImageFormat`].
pub const fn format_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Ico => "ico",
        ImageFormat::Tiff => "tiff",
        _ => "bin",
    }
}

/// Derives a thumbnail storage key by inserting a subdirectory before the file name
/// and replacing the file extension with the target format extension.
///
/// Normalizes any backslashes to forward slashes for cross-platform cloud storage consistency.
///
/// # Examples
///
/// ```rust
/// use image::ImageFormat;
/// use scyph_storage::thumbnail::derive_thumbnail_key_with_format;
///
/// let original = "uploads/users/avatar.png";
/// let thumb = derive_thumbnail_key_with_format(original, "thumbnails", ImageFormat::Jpeg);
/// assert_eq!(thumb, "uploads/users/thumbnails/avatar.jpg");
/// ```
pub fn derive_thumbnail_key_with_format(
    original_key: &str,
    subfolder: &str,
    format: ImageFormat,
) -> String {
    let key_with_folder = derive_thumbnail_key(original_key, subfolder);
    let target_ext = format_extension(format);

    let path = Path::new(&key_with_folder);
    if path.extension().is_some() {
        let mut new_path = path.to_path_buf();
        new_path.set_extension(target_ext);
        new_path.to_string_lossy().replace('\\', "/")
    } else {
        format!("{key_with_folder}.{target_ext}")
    }
}

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
    ) -> impl Future<Output = Result<ThumbnailStoreResult, StorageError>> + Send;
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
