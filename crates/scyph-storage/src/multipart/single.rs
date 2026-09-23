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
///     Ok(Json(file.path))
/// }
/// ```
#[derive(Debug, Clone)]
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
                    "Unsupported file type '{content_type}'. Allowed types: {:?}",
                    C::allowed_mime_types()
                ))
                .into());
            }

            let data = read_field_bytes(field, C::max_size(), &original_name).await?;
            let extension = C::resolve_extension(&content_type);
            let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

            return Ok(FileExtractor::new(
                data,
                content_type,
                path,
                original_name,
            ));
        }

        Err(StorageError::InvalidFile(format!(
            "Required file field '{}' was not found in request",
            C::field_name()
        ))
        .into())
    }
}
