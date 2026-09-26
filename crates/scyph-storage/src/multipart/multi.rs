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
#[derive(Debug, Clone)]
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

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let field_name = field.name().unwrap_or_default();

            let is_matching_field = if target_name.is_empty() || target_name == "file" || target_name == "files" {
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
