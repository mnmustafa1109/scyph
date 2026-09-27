//! Integration tests for scyph-storage thumbnail utilities.

use image::ImageFormat;
use scyph_storage::thumbnail::{
    DEFAULT_THUMBNAIL_DIMENSION, DEFAULT_THUMBNAIL_SUBFOLDER, ThumbnailConfig,
    derive_thumbnail_key, generate_thumbnail, generate_thumbnail_with_format, thumbnail_key,
};

#[test]
fn test_thumbnail_key_replacement() {
    let key = "messages/images/01923f-test.jpg";
    let thumb = thumbnail_key(key, "messages/images/", "messages/thumbnails/");
    assert_eq!(thumb, "messages/thumbnails/01923f-test.jpg");
}

#[test]
fn test_thumbnail_key_fallback() {
    let key = "documents/contract.pdf";
    let thumb = thumbnail_key(key, "messages/images/", "messages/thumbnails/");
    assert_eq!(thumb, format!("documents/{DEFAULT_THUMBNAIL_SUBFOLDER}/contract.pdf"));
}

#[test]
fn test_derive_thumbnail_key_nested() {
    let key = "uploads/avatars/user.png";
    let thumb = derive_thumbnail_key(key, "thumbs");
    assert_eq!(thumb, "uploads/avatars/thumbs/user.png");
}

#[test]
fn test_derive_thumbnail_key_root_level() {
    let root_file = "photo.jpg";
    let thumb_root = derive_thumbnail_key(root_file, "thumbs");
    assert_eq!(thumb_root, "thumbs/photo.jpg");
}

#[test]
fn test_derive_thumbnail_key_windows_separators() {
    let key = "uploads\\avatars\\user.png";
    let thumb = derive_thumbnail_key(key, "thumbs");
    assert_eq!(thumb, "uploads/avatars/thumbs/user.png");
}

#[test]
fn test_thumbnail_config_defaults() {
    let config = ThumbnailConfig::default();
    assert_eq!(config.max_width, DEFAULT_THUMBNAIL_DIMENSION);
    assert_eq!(config.max_height, DEFAULT_THUMBNAIL_DIMENSION);
    assert_eq!(config.format, ImageFormat::Jpeg);
}

#[test]
fn test_thumbnail_config_builder() {
    let config = ThumbnailConfig::new(400, 300).with_format(ImageFormat::Png);
    assert_eq!(config.max_width, 400);
    assert_eq!(config.max_height, 300);
    assert_eq!(config.format, ImageFormat::Png);
}

#[test]
fn test_generate_thumbnail_invalid_data() {
    let invalid_bytes = b"not a real image payload";
    let res = generate_thumbnail(invalid_bytes, 100, 100);
    assert!(res.is_err());
}

#[test]
fn test_generate_thumbnail_with_format_invalid_data() {
    let invalid_bytes = b"corrupted bytes";
    let res = generate_thumbnail_with_format(invalid_bytes, 200, 200, ImageFormat::Png);
    assert!(res.is_err());
}
