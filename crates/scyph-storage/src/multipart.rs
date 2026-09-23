//! Declarative file configuration traits and Axum multipart extractors.
//!
//! Provides type-safe, declarative extractors that validate incoming HTTP file uploads,
//! stream binary payloads safely in chunks to prevent memory exhaustion, and automatically
//! construct time-ordered UUID v7 storage paths.

use axum::{
    body::Bytes,
    extract::{FromRequest, Multipart, Request},
};
use bytes::BytesMut;
use scyph_core::error::AppError;
use std::{marker::PhantomData, ops::Deref, vec::IntoIter};
use uuid::Uuid;

use crate::error::StorageError;

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

/// Axum request extractor for single-file uploads with type-level validation configuration `C`.
///
/// Automatically parses the multipart boundary, verifies the field name matches [`FileConfig::field_name`],
/// validates the Content-Type header against [`FileConfig::allowed_mime_types`], streams binary data
/// chunk-by-chunk with immediate enforcement of [`FileConfig::max_size`], and formats a time-ordered
/// UUID v7 S3 key path.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::{extract::State, response::Json};
/// use scyph_core::error::AppError;
/// use scyph_storage::{FileConfig, FileExtractor, StorageService};
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
///     Ok(Json(file.path))
/// }
/// ```
pub struct FileExtractor<C: FileConfig> {
    /// Zero-copy byte buffer containing the uploaded file content.
    pub data: Bytes,
    /// MIME content-type of the file (e.g. `"image/png"`).
    pub content_type: String,
    /// Generated S3 destination path formatted as `{storage_path}/{uuid_v7}.{ext}`.
    pub path: String,
    /// Original filename sent by the client's browser (e.g. `"resume.pdf"`).
    pub original_name: String,
    _config: PhantomData<C>,
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

        while let Some(mut field) = multipart
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
                    "Unsupported file type '{content_type}'. Allowed types: {:?}",
                    C::allowed_mime_types()
                ))
                .into());
            }

            let mut buffer = BytesMut::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| StorageError::InvalidFile(format!("Error reading file chunk: {e}")))?
            {
                if buffer.len() + chunk.len() > C::max_size() {
                    return Err(StorageError::FileTooLarge(format!(
                        "File exceeds maximum allowed size of {} bytes",
                        C::max_size()
                    ))
                    .into());
                }
                buffer.extend_from_slice(&chunk);
            }

            let data = buffer.freeze();
            let extension = C::resolve_extension(&content_type);
            let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

            return Ok(FileExtractor {
                data,
                content_type,
                path,
                original_name,
                _config: PhantomData,
            });
        }

        Err(StorageError::InvalidFile(format!(
            "Required file field '{}' was not found in request",
            C::field_name()
        ))
        .into())
    }
}

/// Represents an individual validated file extracted from a multi-file upload payload.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// Zero-copy byte buffer containing the uploaded file content.
    pub data: Bytes,
    /// MIME content-type of the file.
    pub content_type: String,
    /// Generated S3 destination path formatted as `{storage_path}/{uuid_v7}.{ext}`.
    pub path: String,
    /// Original filename submitted by the client.
    pub original_name: String,
}

/// Axum request extractor for batch/multi-file uploads with type-level validation configuration `C`.
///
/// Matches field names matching the configured name, plural suffixes, and array notation
/// (e.g. `attachment`, `attachments`, `attachment[]`, `files[]`).
///
/// Implements [`Deref<Target = [ExtractedFile]>`](Deref) and [`IntoIterator<Item = ExtractedFile>`](IntoIterator)
/// for ergonomic iteration and indexing.
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
/// ) -> Result<Json<usize>, AppError> {
///     println!("Uploaded {} photos", upload.len());
///     for file in upload {
///         // Process individual ExtractedFile
///     }
///     Ok(Json(200))
/// }
/// ```
pub struct MultiFileExtractor<C: FileConfig> {
    /// Vector of extracted and validated files.
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

        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let field_name = field.name().unwrap_or_default();

            let is_matching_field = field_name == target_name
                || field_name == target_plural
                || field_name == target_array
                || field_name == target_plural_array
                || field_name == "files"
                || field_name == "files[]"
                || field_name == "file"
                || field_name == "file[]";

            if !is_matching_field && field.file_name().is_none() {
                continue;
            }

            if !is_matching_field && field.file_name().is_some() && !target_name.is_empty() {
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
                    "File '{original_name}' has unsupported type '{content_type}'. Allowed: {:?}",
                    C::allowed_mime_types()
                ))
                .into());
            }

            let mut buffer = BytesMut::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| StorageError::InvalidFile(format!("Error reading file chunk: {e}")))?
            {
                if buffer.len() + chunk.len() > C::max_size() {
                    return Err(StorageError::FileTooLarge(format!(
                        "File '{original_name}' exceeds maximum allowed size of {} bytes",
                        C::max_size()
                    ))
                    .into());
                }
                buffer.extend_from_slice(&chunk);
            }

            let data = buffer.freeze();
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
