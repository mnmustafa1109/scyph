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
    assert_eq!(
        thumb,
        format!("documents/{DEFAULT_THUMBNAIL_SUBFOLDER}/contract.pdf")
    );
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

#[test]
fn test_thumbnail_config_mime_and_extension() {
    let jpeg_config = ThumbnailConfig::default();
    assert_eq!(jpeg_config.content_type(), "image/jpeg");
    assert_eq!(jpeg_config.extension(), "jpg");

    let webp_config = ThumbnailConfig::default().with_format(ImageFormat::WebP);
    assert_eq!(webp_config.content_type(), "image/webp");
    assert_eq!(webp_config.extension(), "webp");

    let png_config = ThumbnailConfig::default().with_format(ImageFormat::Png);
    assert_eq!(png_config.content_type(), "image/png");
    assert_eq!(png_config.extension(), "png");
}

#[test]
fn test_derive_thumbnail_key_with_format() {
    use scyph_storage::thumbnail::derive_thumbnail_key_with_format;

    let key = "uploads/avatars/user.png";
    let thumb = derive_thumbnail_key_with_format(key, "thumbnails", ImageFormat::WebP);
    assert_eq!(thumb, "uploads/avatars/thumbnails/user.webp");

    let config = ThumbnailConfig::default().with_format(ImageFormat::Jpeg);
    let thumb_config = config.derive_key("uploads/avatars/user.png", "thumbnails");
    assert_eq!(thumb_config, "uploads/avatars/thumbnails/user.jpg");

    let no_ext = "uploads/avatars/user";
    let thumb_no_ext = config.derive_key(no_ext, "thumbs");
    assert_eq!(thumb_no_ext, "uploads/avatars/thumbs/user.jpg");
}

#[tokio::test]
async fn test_storage_thumbnail_ext() {
    use image::{ImageBuffer, Rgb};
    use scyph_storage::{InMemoryStorageService, StorageThumbnailExt};

    let storage = InMemoryStorageService::new();

    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();

    let orig_key = "gallery/photo.png";
    let config = ThumbnailConfig::new(5, 5).with_format(ImageFormat::Jpeg);

    let res = storage
        .store_with_thumbnail(orig_key, "image/png", bytes::Bytes::from(bytes), config)
        .await
        .unwrap();

    assert_eq!(res.original_key, "gallery/photo.png");
    assert_eq!(res.thumbnail_key, "gallery/thumbnails/photo.jpg");

    assert!(storage.contains("gallery/photo.png"));
    assert!(storage.contains("gallery/thumbnails/photo.jpg"));
    assert_eq!(storage.count(), 2);
}

#[test]
fn test_extracted_file_thumbnail_helper() {
    use image::{ImageBuffer, Rgb};
    use scyph_storage::ExtractedFile;

    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(20, 20);
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();

    let file = ExtractedFile {
        data: bytes::Bytes::from(bytes),
        content_type: "image/png".to_string(),
        path: "avatars/01923-user.png".to_string(),
        original_name: "me.png".to_string(),
    };

    let config = ThumbnailConfig::new(10, 10).with_format(ImageFormat::WebP);
    let (thumb_key, thumb_data) = file.thumbnail(config).unwrap();

    assert_eq!(thumb_key, "avatars/thumbnails/01923-user.webp");
    assert!(!thumb_data.is_empty());
}
