//! Image decoding, scaling, and re-encoding routines.

use std::io::Cursor;

use image::{ImageFormat, ImageReader};

use crate::error::StorageError;

/// Maximum allowed decoded pixel buffer size (64 MiB).
///
/// Caps the memory `image` will allocate while decoding, so a decompression bomb
/// is rejected instead of exhausting the allocator.
const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;

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
pub async fn generate_thumbnail(
    data: impl Into<bytes::Bytes>,
    max_width: u32,
    max_height: u32,
) -> Result<Vec<u8>, StorageError> {
    generate_thumbnail_with_format(data, max_width, max_height, ImageFormat::Jpeg).await
}

/// Generates a thumbnail buffer encoded with a specified destination format.
///
/// Decoding and resizing delegate to [`image`], which documents CPU-bound work;
/// this function therefore runs it on [`tokio::task::spawn_blocking`] so the async
/// runtime workers are never blocked by large image processing.
///
/// Protects against decompression bomb vulnerabilities by enforcing strict reader
/// limits and clamps bounding dimensions to at least 1 pixel.
///
/// # Arguments
///
/// * `data` - Raw bytes of the source image (moved into the blocking task; cheap if `Bytes`).
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
pub async fn generate_thumbnail_with_format(
    data: impl Into<bytes::Bytes>,
    max_width: u32,
    max_height: u32,
    format: ImageFormat,
) -> Result<Vec<u8>, StorageError> {
    // Clone into an owned buffer: spawn_blocking needs 'static data, and
    // `Bytes` makes this cheap when the caller already holds one.
    let data = data.into();

    tokio::task::spawn_blocking(move || {
        let bound_w = max_width.max(1);
        let bound_h = max_height.max(1);

        let mut reader = ImageReader::new(Cursor::new(&data[..]))
            .with_guessed_format()
            .map_err(|e| {
                StorageError::validation(format!("Failed to inspect image format: {e}"), vec![])
            })?;

        let mut limits = image::Limits::default();
        limits.max_alloc = Some(MAX_DECODE_BYTES);
        reader.limits(limits);

        let dynamic_img = reader.decode().map_err(|e| {
            StorageError::validation(format!("Failed to decode image: {e}"), vec![])
        })?;

        let thumbnail = dynamic_img.thumbnail(bound_w, bound_h);

        let mut output_buf = Vec::new();
        thumbnail
            .write_to(&mut Cursor::new(&mut output_buf), format)
            .map_err(|e| {
                StorageError::Internal(format!("Failed to encode thumbnail image: {e}"))
            })?;

        Ok(output_buf)
    })
    .await
    .map_err(|e| StorageError::Internal(format!("Thumbnail generation task panicked: {e}")))?
}
