//! Thumbnail configuration and format metadata.

use image::ImageFormat;

use super::{generator::generate_thumbnail_with_format, key::derive_thumbnail_key_with_format};
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
