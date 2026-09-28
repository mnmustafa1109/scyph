//! Multiple-file Axum request extractor.

use std::{marker::PhantomData, ops::Deref, vec::IntoIter};

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

/// Represents an individual validated file extracted from a multi-file upload payload.
///
/// Each `ExtractedFile` holds the complete buffered content of one file along with its
/// metadata. Use `data`, `content_type`, `path`, and `original_name` to store and track the file.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// Zero-copy byte buffer containing the complete uploaded file content.
    ///
    /// Pass directly to [`StorageService::store`](crate::traits::StorageService::store).
    pub data: Bytes,

    /// MIME content-type of the file as declared in the multipart `Content-Type` header.
    pub content_type: String,

    /// Generated S3 destination key formatted as `{storage_path}/{uuid_v7}.{ext}`.
    ///
    /// Each file in the batch gets its own unique UUIDv7-based key. Store this path in
    /// your database to retrieve or generate signed URLs later.
    pub path: String,

    /// Original filename submitted by the client (e.g., `"photo1.jpg"`).
    ///
    /// The original name is **not** used in the storage key. It may be useful for display
    /// purposes or audit logs.
    pub original_name: String,
}

#[cfg(feature = "image")]
impl ExtractedFile {
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

/// Axum request extractor for batch/multi-file uploads with type-level validation configuration `C`.
///
/// Implements [`FromRequest`](axum::extract::FromRequest) and collects all matching file fields
/// from a multipart request into a `Vec<ExtractedFile>`. Each file is individually validated
/// for MIME type, size, and magic bytes according to the [`FileConfig`] type parameter.
///
/// ## Field Name Matching
///
/// `MultiFileExtractor` accepts multiple naming conventions for the multipart field name.
/// For a `FileConfig` with `field_name() = "photo"`, it matches:
///
/// | Form field name | Matched? |
/// |---|---|
/// | `photo` | ✓ |
/// | `photos` | ✓ (plural suffix) |
/// | `photo[]` | ✓ (array notation) |
/// | `photos[]` | ✓ (plural array notation) |
/// | `image` | ✗ |
///
/// ## Rejection Behavior
///
/// - Returns `422 Unprocessable Entity` if any single file fails MIME type validation or magic bytes
/// - Returns `422 Unprocessable Entity` if any single file exceeds [`max_size()`](FileConfig::max_size)
/// - Returns `422 Unprocessable Entity` if more than [`max_files()`](FileConfig::max_files) files are submitted
/// - Returns `400 Bad Request` if no matching files are found at all
///
/// ## Iteration
///
/// `MultiFileExtractor` implements [`Deref<Target = [ExtractedFile]>`](Deref) and
/// [`IntoIterator<Item = ExtractedFile>`](IntoIterator) for ergonomic iteration and indexing.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::response::Json;
/// use scyph_core::error::AppError;
/// use scyph_storage::{FileConfig, MultiFileExtractor};
///
/// pub struct GalleryUpload;
/// impl FileConfig for GalleryUpload {
///     fn field_name() -> &'static str { "photo" }
///     fn max_size() -> usize { 5 * 1024 * 1024 }
///     fn max_files() -> usize { 10 }
///     fn allowed_mime_types() -> Vec<&'static str> { vec!["image/jpeg", "image/png"] }
///     fn storage_path() -> &'static str { "gallery" }
/// }
///
/// async fn upload_gallery_handler(
///     upload: MultiFileExtractor<GalleryUpload>,
/// ) -> Result<Json<Vec<String>>, AppError> {
///     let paths: Vec<String> = upload.files.iter().map(|f| f.path.clone()).collect();
///     Ok(Json(paths))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MultiFileExtractor<C: FileConfig> {
    /// Vector of extracted and validated files, one per matching multipart field entry.
    pub files: Vec<ExtractedFile>,
    _config: PhantomData<C>,
}

impl<C: FileConfig> Deref for MultiFileExtractor<C> {
    type Target = [ExtractedFile];

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl<C: FileConfig> IntoIterator for MultiFileExtractor<C> {
    type Item = ExtractedFile;
    type IntoIter = IntoIter<ExtractedFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.files.into_iter()
    }
}

impl<S, C> FromRequest<S> for MultiFileExtractor<C>
where
    S: Send + Sync,
    C: FileConfig,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart read error: {e}")))?;

        let mut files = Vec::new();
        let target_name = C::field_name();
        let target_plural = format!("{target_name}s");
        let target_array = format!("{target_name}[]");
        let target_plural_array = format!("{target_name}s[]");

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let field_name = field.name().unwrap_or_default();

            let is_matching_field =
                if target_name.is_empty() || target_name == "file" || target_name == "files" {
                    field_name == "file"
                        || field_name == "files"
                        || field_name == "file[]"
                        || field_name == "files[]"
                        || target_name.is_empty()
                } else {
                    field_name == target_name
                        || field_name == target_plural
                        || field_name == target_array
                        || field_name == target_plural_array
                };

            if !is_matching_field {
                continue;
            }

            let original_name = match field.file_name() {
                Some(name) => name.to_string(),
                None => continue,
            };

            if files.len() >= C::max_files() {
                return Err(StorageError::TooManyFiles(format!(
                    "Exceeded maximum allowed limit of {} files per request",
                    C::max_files()
                ))
                .into());
            }

            let content_type = field
                .content_type()
                .map(|ct| ct.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            if !C::allowed_mime_types().contains(&content_type.as_str()) {
                return Err(StorageError::UnsupportedMediaType(format!(
                    "File '{original_name}' for field '{}' has unsupported Content-Type '{content_type}'. Allowed MIME types: {:?}",
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

            files.push(ExtractedFile {
                data,
                content_type,
                path,
                original_name,
            });
        }

        if files.is_empty() {
            return Err(StorageError::InvalidFile(format!(
                "No valid files found for field '{}' in request",
                C::field_name()
            ))
            .into());
        }

        Ok(MultiFileExtractor {
            files,
            _config: PhantomData,
        })
    }
}
