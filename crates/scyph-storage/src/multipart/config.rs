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
