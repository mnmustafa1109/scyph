//! Image thumbnail generation utilities for storage pipelines.
//!
//! Provides configurable image decoding, aspect-ratio-preserving resizing,
//! format encoding, and storage path key derivation for thumbnail assets.

use std::io::Cursor;
use std::path::Path;

use image::{ImageFormat, ImageReader};

use crate::error::StorageError;

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
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| StorageError::validation(format!("Failed to inspect image format: {e}"), vec![]))?;

    let dynamic_img = reader.decode().map_err(|e| {
        StorageError::validation(format!("Failed to decode image: {e}"), vec![])
    })?;

    let thumbnail = dynamic_img.thumbnail(max_width, max_height);

    let mut output_buf = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut output_buf), format)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to encode thumbnail image: {e}"))
        })?;

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
    let path = Path::new(original_key);
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
        format!("{}/{}", subfolder, original_key)
    }
}
