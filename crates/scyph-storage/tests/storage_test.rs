use bytes::Bytes;
use scyph_storage::{FileConfig, InMemoryStorageService, StorageService};

#[tokio::test]
async fn test_memory_storage_crud() {
    let storage = InMemoryStorageService::new();
    let path = "avatars/01.png";
    let data = Bytes::from_static(b"binary content");

    // Store
    let stored_path = storage
        .store(path, "image/png", data.clone())
        .await
        .unwrap();
    assert_eq!(stored_path, path);
    assert_eq!(storage.count(), 1);
    assert!(storage.contains(path));

    // Retrieve
    let retrieved = storage.retrieve(path).await.unwrap();
    assert_eq!(retrieved, data);

    // View URL
    let view_url = storage.get_view_url(path, 3600).await.unwrap();
    assert!(view_url.contains("view=true"));

    // Download URL with sanitization
    let download_url = storage
        .get_download_url(path, "my\"file\r\n.png", 3600)
        .await
        .unwrap();
    assert!(download_url.contains("download=myfile.png"));

    // Delete
    storage.delete(path).await.unwrap();
    assert_eq!(storage.count(), 0);
    assert!(!storage.contains(path));
    assert!(storage.retrieve(path).await.is_err());
}

struct TestConfig;
impl FileConfig for TestConfig {
    fn field_name() -> &'static str {
        "test"
    }
    fn max_size() -> usize {
        1024
    }
    fn allowed_mime_types() -> Vec<&'static str> {
        vec!["image/png", "application/pdf"]
    }
    fn storage_path() -> &'static str {
        "tests"
    }
}

#[test]
fn test_magic_bytes_validation() {
    let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
    let pdf_bytes = b"%PDF-1.4 header contents";
    let fake_png = b"NOT_A_PNG_FILE";

    assert!(TestConfig::verify_magic_bytes(&png_bytes, "image/png"));
    assert!(!TestConfig::verify_magic_bytes(fake_png, "image/png"));
    assert!(TestConfig::verify_magic_bytes(pdf_bytes, "application/pdf"));
}

#[test]
fn test_resolve_extensions() {
    assert_eq!(TestConfig::resolve_extension("image/png"), "png");
    assert_eq!(TestConfig::resolve_extension("application/pdf"), "pdf");
    assert_eq!(TestConfig::resolve_extension("custom/binary"), "bin");
}
