//! Single-file Axum request extractor.

use std::marker::PhantomData;

use axum::{
    body::Bytes,
    extract::{FromRequest, Multipart, Request},
};
use scyph_core::error::AppError;
use uuid::Uuid;

use crate::{
    error::StorageError,
    multipart::{config::FileConfig, util::read_field_bytes},
};

/// Axum request extractor for single required file uploads with type-level validation configuration `C`.
///
/// Implements [`FromRequest`](axum::extract::FromRequest) — Axum calls it automatically when
/// used as a handler parameter. It parses the multipart boundary, locates the field matching
/// [`FileConfig::field_name`], validates Content-Type and magic bytes, buffers the data with
/// size enforcement, and generates a time-ordered UUIDv7 S3 key.
///
/// ## Type Parameter
///
/// `C` must implement [`FileConfig`]. Use a zero-sized marker struct:
///
/// ```rust
/// use scyph_storage::FileConfig;
///
/// pub struct AvatarUpload; // zero-sized, zero runtime cost
///
/// impl FileConfig for AvatarUpload {
///     fn field_name() -> &'static str { "avatar" }
///     fn max_size() -> usize { 2 * 1024 * 1024 }
///     fn allowed_mime_types() -> Vec<&'static str> { vec!["image/jpeg", "image/png"] }
///     fn storage_path() -> &'static str { "avatars" }
/// }
/// ```
///
/// ## Validation Steps (in order)
///
/// 1. Parse multipart boundary from `Content-Type: multipart/form-data; boundary=...`
/// 2. Iterate fields until one matching [`FileConfig::field_name()`](FileConfig::field_name) is found
/// 3. Verify the field carries a `filename` (rejects non-file fields)
/// 4. Check `Content-Type` header against [`allowed_mime_types()`](FileConfig::allowed_mime_types)
/// 5. Stream data chunks, abort immediately if accumulated size exceeds [`max_size()`](FileConfig::max_size)
/// 6. If [`enforce_magic_bytes()`](FileConfig::enforce_magic_bytes) is `true`, verify binary signature
/// 7. Generate key: `{storage_path()}/{uuid_v7}.{extension}`
///
/// ## Rejection Behavior
///
/// Returns `AppError` (HTTP 400/422) if any validation step fails. The request body is consumed.
/// Use [`OptionalFileExtractor`](crate::multipart::OptionalFileExtractor) if the file field is optional.
///
/// ## Full Handler Example
///
/// ```rust,no_run
/// use axum::response::Json;
/// use scyph_core::error::AppError;
/// use scyph_storage::{FileConfig, FileExtractor};
///
/// pub struct DocumentUpload;
/// impl FileConfig for DocumentUpload {
///     fn field_name() -> &'static str { "document" }
///     fn max_size() -> usize { 10 * 1024 * 1024 }
///     fn allowed_mime_types() -> Vec<&'static str> { vec!["application/pdf"] }
///     fn storage_path() -> &'static str { "documents" }
/// }
///
/// async fn upload_doc_handler(
///     file: FileExtractor<DocumentUpload>,
/// ) -> Result<Json<String>, AppError> {
///     // file.path is a ready-to-use S3 key, e.g., "documents/018f2d5e-....pdf"
///     Ok(Json(file.path))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileExtractor<C: FileConfig> {
    /// Zero-copy byte buffer containing the complete uploaded file content.
    ///
    /// The entire file is buffered in memory after passing size validation.
    /// Pass this directly to [`StorageService::store`](crate::traits::StorageService::store).
    pub data: Bytes,

    /// MIME content-type of the file as declared in the multipart `Content-Type` header.
    ///
    /// Always a string from [`FileConfig::allowed_mime_types()`](FileConfig::allowed_mime_types),
    /// defaulting to `"application/octet-stream"` if the header was absent.
    pub content_type: String,

    /// Generated S3 destination key formatted as `{storage_path}/{uuid_v7}.{ext}`.
    ///
    /// This is a relative path/key suitable for use as the `path` argument to
    /// [`StorageService::store`](crate::traits::StorageService::store).
    /// Example: `"avatars/018f2d5e-4a6c-7000-8000-000000000001.png"`
    pub path: String,

    /// Original filename submitted by the client's browser (e.g., `"resume.pdf"`, `"photo.jpg"`).
    ///
    /// This is the raw filename from the multipart `Content-Disposition` header. It is
    /// **not** used for the S3 key (a UUID v7 path is generated instead), but may be useful
    /// for display purposes or audit logging.
    pub original_name: String,

    _config: PhantomData<C>,
}

impl<C: FileConfig> FileExtractor<C> {
    pub(crate) fn new(
        data: Bytes,
        content_type: String,
        path: String,
        original_name: String,
    ) -> Self {
        Self {
            data,
            content_type,
            path,
            original_name,
            _config: PhantomData,
        }
    }

    /// Generates a thumbnail buffer and derives its storage key using the provided [`ThumbnailConfig`](crate::thumbnail::ThumbnailConfig).
    ///
    /// Derives the thumbnail key path placing it under `thumbnails/` and replacing the extension with the target format.
    ///
    /// # Returns
    ///
    /// Returns a tuple `(thumbnail_key, thumbnail_bytes)`.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::ValidationError`] if decoding fails, or [`StorageError::Internal`] if re-encoding fails.
    #[cfg(feature = "image")]
    pub fn thumbnail(
        &self,
        config: crate::thumbnail::ThumbnailConfig,
    ) -> Result<(String, Bytes), StorageError> {
        let thumb_bytes = config.generate(&self.data)?;
        let thumb_key =
            config.derive_key(&self.path, crate::thumbnail::DEFAULT_THUMBNAIL_SUBFOLDER);
        Ok((thumb_key, Bytes::from(thumb_bytes)))
    }
}

impl<S, C> FromRequest<S> for FileExtractor<C>
where
    S: Send + Sync,
    C: FileConfig,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart read error: {e}")))?;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let name = field.name().unwrap_or_default();
            if name != C::field_name() {
                continue;
            }

            let original_name = match field.file_name() {
                Some(name) => name.to_string(),
                None => {
                    return Err(StorageError::InvalidFile(format!(
                        "Field '{name}' must be a valid file"
                    ))
                    .into());
                }
            };

            let content_type = field
                .content_type()
                .map(|ct| ct.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            if !C::allowed_mime_types().contains(&content_type.as_str()) {
                return Err(StorageError::UnsupportedMediaType(format!(
                    "Unsupported Content-Type '{content_type}' for field '{}'. Allowed MIME types: {:?}",
                    C::field_name(),
                    C::allowed_mime_types()
                ))
                .into());
            }

            let data = read_field_bytes(field, C::max_size(), &original_name).await?;

            if C::enforce_magic_bytes() && !C::verify_magic_bytes(&data, &content_type) {
                return Err(StorageError::UnsupportedMediaType(format!(
                    "File '{original_name}' content signature does not match declared MIME type '{content_type}'"
                ))
                .into());
            }

            let extension = C::resolve_extension(&content_type);
            let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

            return Ok(FileExtractor::new(data, content_type, path, original_name));
        }

        Err(StorageError::InvalidFile(format!(
            "Required file field '{}' was not found in request",
            C::field_name()
        ))
        .into())
    }
}
