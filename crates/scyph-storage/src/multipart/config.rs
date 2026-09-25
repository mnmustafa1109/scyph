//! Declarative file upload configuration traits.

/// Trait defining declarative validation constraints and storage routing for file uploads.
///
/// Implement this trait on zero-sized structs to define upload policies for specific use cases
/// (e.g. user avatars, PDF contracts, message attachments).
///
/// # Examples
///
/// ```rust
/// use scyph_storage::FileConfig;
///
/// pub struct AvatarConfig;
///
/// impl FileConfig for AvatarConfig {
///     fn field_name() -> &'static str { "avatar" }
///     fn max_size() -> usize { 2 * 1024 * 1024 } // 2 MB
///     fn allowed_mime_types() -> Vec<&'static str> {
///         vec!["image/jpeg", "image/png", "image/webp"]
///     }
///     fn storage_path() -> &'static str { "avatars" }
/// }
/// ```
pub trait FileConfig: Send + Sync + 'static {
    /// The expected multipart form field name in the HTML form/payload (e.g., `"avatar"`, `"attachment"`, `"document"`).
    fn field_name() -> &'static str;

    /// Maximum allowed file size in bytes for a single file.
    fn max_size() -> usize;

    /// Whitelist of allowed MIME Content-Type headers (e.g., `vec!["image/jpeg", "image/png"]`).
    fn allowed_mime_types() -> Vec<&'static str>;

    /// S3 folder directory prefix where the file should be routed (e.g., `"avatars"`, `"contracts/documents"`).
    fn storage_path() -> &'static str;

    /// Maximum number of files permitted in a single multi-file upload batch (defaults to 10).
    fn max_files() -> usize {
        10
    }

    /// Whether to inspect uploaded binary data against standard magic byte signatures (defaults to `true`).
    ///
    /// When enabled, uploads declaring MIME types like `image/png` or `application/pdf` must have matching
    /// binary file header signatures, preventing MIME-spoofing attacks.
    fn enforce_magic_bytes() -> bool {
        true
    }

    /// Validates whether the binary payload matches the expected file signature for `content_type`.
    ///
    /// Provides built-in magic byte verification for JPEG, PNG, GIF, WEBP, PDF, ZIP, and Office documents.
    /// Can be overridden for custom binary formats.
    fn verify_magic_bytes(data: &[u8], content_type: &str) -> bool {
        match content_type {
            "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            "image/jpeg" => data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF,
            "image/gif" => data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a"),
            "image/webp" => data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP",
            "application/pdf" => data.starts_with(b"%PDF-"),
            "application/zip"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
            }
            "application/x-rar-compressed" => data.starts_with(&[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07]),
            _ => true,
        }
    }

    /// Resolves the canonical file extension string for a given MIME content-type.
    ///
    /// Provides built-in mappings for common image, document, and archive types. Can be overridden
    /// for custom application-specific MIME types.
    fn resolve_extension(content_type: &str) -> &'static str {
        match content_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "application/pdf" => "pdf",
            "application/msword" => "doc",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
            "application/vnd.ms-excel" => "xls",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
            "application/zip" => "zip",
            "application/x-rar-compressed" => "rar",
            "text/plain" => "txt",
            "text/html" => "html",
            "text/csv" => "csv",
            "application/json" => "json",
            _ => "bin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
