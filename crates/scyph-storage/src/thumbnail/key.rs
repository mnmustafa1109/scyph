//! Storage key path derivation and folder substitution utilities.

use std::path::Path;

use image::ImageFormat;

use super::config::{format_extension, DEFAULT_THUMBNAIL_SUBFOLDER};

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
