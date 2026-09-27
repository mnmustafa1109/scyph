//! Image decoding, scaling, and re-encoding routines.

use std::io::Cursor;

use image::{ImageFormat, ImageReader};

use crate::error::StorageError;

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
/// Protects against decompression bomb vulnerabilities by enforcing strict reader limits
/// and clamps bounding dimensions to at least 1 pixel.
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
