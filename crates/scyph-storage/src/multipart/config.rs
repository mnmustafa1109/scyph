//! Declarative file upload configuration traits.

/// Trait defining declarative validation constraints and storage routing for file uploads.
///
/// Implement this trait on zero-sized structs to define upload policies for specific use cases.
/// The trait is used as a type-level configuration parameter for [`FileExtractor<C>`](crate::multipart::FileExtractor),
/// [`MultiFileExtractor<C>`](crate::multipart::MultiFileExtractor), and
/// [`OptionalFileExtractor<C>`](crate::multipart::OptionalFileExtractor).
///
/// ## Design Philosophy
///
/// Each unique upload use case gets its own zero-sized `FileConfig` implementor. This provides:
/// - **Compile-time type safety** — wrong extractor type in a handler is a compile error
/// - **Zero runtime overhead** — no heap allocation for the config type itself
/// - **Self-documenting code** — upload constraints are co-located with the type definition
///
/// ## Validation Pipeline
///
/// When `FileExtractor<C>` processes a multipart request, it applies constraints in this order:
///
/// 1. Find the multipart field matching [`field_name()`](Self::field_name)
/// 2. Verify the field has a filename (rejects non-file fields)
/// 3. Check `Content-Type` header against [`allowed_mime_types()`](Self::allowed_mime_types)
/// 4. Stream and buffer file data, enforcing [`max_size()`](Self::max_size) per chunk
/// 5. If [`enforce_magic_bytes()`](Self::enforce_magic_bytes) is `true`, verify binary header via [`verify_magic_bytes()`](Self::verify_magic_bytes)
/// 6. Generate a collision-free UUID v7 S3 key: `{storage_path()}/{uuid_v7}.{ext}`
///
/// ## Examples
///
/// ### Basic image upload
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
///
/// ### Document upload with disabled magic byte check
///
/// ```rust
/// use scyph_storage::FileConfig;
///
/// pub struct ContractUpload;
///
/// impl FileConfig for ContractUpload {
///     fn field_name() -> &'static str { "contract" }
///     fn max_size() -> usize { 20 * 1024 * 1024 } // 20 MB
///     fn allowed_mime_types() -> Vec<&'static str> {
///         vec!["application/pdf"]
///     }
///     fn storage_path() -> &'static str { "contracts/documents" }
///     fn enforce_magic_bytes() -> bool { false } // skip magic bytes for third-party PDFs
/// }
/// ```
///
/// ### Multi-file gallery with custom file limit
///
/// ```rust
/// use scyph_storage::FileConfig;
///
/// pub struct GalleryUpload;
///
/// impl FileConfig for GalleryUpload {
///     fn field_name() -> &'static str { "photo" }
///     fn max_size() -> usize { 8 * 1024 * 1024 } // 8 MB per file
///     fn max_files() -> usize { 20 }              // up to 20 files per request
///     fn allowed_mime_types() -> Vec<&'static str> {
///         vec!["image/jpeg", "image/png", "image/webp", "image/gif"]
///     }
///     fn storage_path() -> &'static str { "gallery/photos" }
/// }
/// ```
pub trait FileConfig: Send + Sync + 'static {
    /// The expected multipart form field name in the HTML form/payload.
    ///
    /// For example, a form field `<input type="file" name="avatar">` has field name `"avatar"`.
    /// Common values: `"avatar"`, `"attachment"`, `"document"`, `"photo"`, `"file"`.
    fn field_name() -> &'static str;

    /// Maximum allowed file size in bytes for a single file.
    ///
    /// Enforced during streaming chunk accumulation — the upload is aborted as soon as
    /// the accumulated byte count exceeds this threshold, preventing OOM from large uploads.
    ///
    /// Common values:
    /// - `2 * 1024 * 1024` — 2 MB (typical avatar)
    /// - `10 * 1024 * 1024` — 10 MB (document)
    /// - `100 * 1024 * 1024` — 100 MB (video)
    fn max_size() -> usize;

    /// Whitelist of allowed MIME Content-Type headers.
    ///
    /// Matched directly against the `Content-Type` field header from the multipart request.
    /// Only exact string matches are accepted — no wildcards or prefix matching.
    ///
    /// # Note on Header Matching
    ///
    /// Matching is performed against the incoming `Content-Type` header string exactly.
    /// Ensure expected client headers match the listed strings (e.g. `"image/png"` not `"image/PNG"`).
    fn allowed_mime_types() -> Vec<&'static str>;

    /// S3 folder directory prefix where the file should be stored.
    ///
    /// The final S3 key is generated as `{storage_path}/{uuid_v7}.{ext}`.
    /// Use forward-slash delimiters for logical folder hierarchy (e.g., `"users/avatars"`,
    /// `"contracts/2024/q1"`). Do not include a leading or trailing slash.
    fn storage_path() -> &'static str;

    /// Maximum number of files permitted in a single multi-file upload batch (defaults to 10).
    ///
    /// Only enforced by [`MultiFileExtractor`](crate::multipart::MultiFileExtractor).
    fn max_files() -> usize {
        10
    }

    /// Whether to inspect uploaded binary data against standard magic byte signatures (defaults to `true`).
    ///
    /// When enabled, uploads declaring MIME types like `image/png` or `application/pdf` must have matching
    /// binary file header signatures, preventing MIME-spoofing attacks where a client uploads a
    /// malicious file (e.g., a PHP script) with a fake `Content-Type: image/jpeg` header.
    ///
    /// Disable this for MIME types without well-known magic bytes, or when processing files from
    /// trusted internal sources where spoofing is not a concern.
    fn enforce_magic_bytes() -> bool {
        true
    }

    /// Validates whether the binary payload matches the expected file signature for `content_type`.
    ///
    /// Provides built-in magic byte verification for common formats. Can be overridden to add
    /// custom binary format validation for application-specific MIME types.
    ///
    /// ## Built-in Supported Types
    ///
    /// | MIME Type | Magic Bytes / Signature |
    /// |---|---|
    /// | `image/png` | `\x89PNG\r\n\x1a\n` (8 bytes) |
    /// | `image/jpeg` | `\xFF\xD8\xFF` (3 bytes) |
    /// | `image/gif` | `GIF87a` or `GIF89a` |
    /// | `image/webp` | `RIFF....WEBP` (12 bytes) |
    /// | `application/pdf` | `%PDF-` |
    /// | `application/zip` | `PK\x03\x04` |
    /// | `.docx`, `.xlsx` | `PK\x03\x04` (ZIP-based Office formats) |
    /// | `application/x-rar-compressed` | `Rar!\x1A\x07` |
    /// | All other types | Always returns `true` |
    fn verify_magic_bytes(data: &[u8], content_type: &str) -> bool {
        match content_type {
            "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            "image/jpeg" => {
                data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF
            }
            "image/gif" => data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a"),
            "image/webp" => data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP",
            "application/pdf" => data.starts_with(b"%PDF-"),
            "application/zip"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
            }
            "application/x-rar-compressed" => {
                data.starts_with(&[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07])
            }
            _ => true,
        }
    }

    /// Resolves the canonical file extension string for a given MIME content-type.
    ///
    /// Used when generating the S3 destination key path to append the correct extension.
    /// Provides built-in mappings for common image, document, and archive types.
    /// Override this method to add mappings for custom application-specific MIME types.
    ///
    /// Returns `"bin"` for unrecognized MIME types.
    ///
    /// ## Built-in Mappings
    ///
    /// | MIME Type | Extension |
    /// |---|---|
    /// | `image/jpeg` | `jpg` |
    /// | `image/png` | `png` |
    /// | `image/gif` | `gif` |
    /// | `image/webp` | `webp` |
    /// | `image/svg+xml` | `svg` |
    /// | `application/pdf` | `pdf` |
    /// | `application/msword` | `doc` |
    /// | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` | `docx` |
    /// | `application/vnd.ms-excel` | `xls` |
    /// | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` | `xlsx` |
    /// | `application/zip` | `zip` |
    /// | `application/x-rar-compressed` | `rar` |
    /// | `text/plain` | `txt` |
    /// | `text/html` | `html` |
    /// | `text/csv` | `csv` |
    /// | `application/json` | `json` |
    /// | *(other)* | `bin` |
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
