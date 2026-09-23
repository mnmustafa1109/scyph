//! Optional single-file Axum request extractor.

use axum::extract::{FromRequest, Multipart, Request};
use scyph_core::error::AppError;
use uuid::Uuid;

use crate::{
    error::StorageError,
    multipart::{config::FileConfig, single::FileExtractor, util::read_field_bytes},
};

/// Axum request extractor for optional single-file uploads with type-level validation configuration `C`.
///
/// Wraps an `Option<FileExtractor<C>>`. If the multipart request contains the specified field,
/// it parses and validates the file as `Some(FileExtractor<C>)`. If the field is omitted,
/// it evaluates to `None` without rejecting the request.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::response::Json;
/// use scyph_core::error::AppError;
/// use scyph_storage::{FileConfig, OptionalFileExtractor};
///
/// pub struct AvatarConfig;
/// impl FileConfig for AvatarConfig {
///     fn field_name() -> &'static str { "avatar" }
///     fn max_size() -> usize { 2 * 1024 * 1024 }
///     fn allowed_mime_types() -> Vec<&'static str> { vec!["image/jpeg", "image/png"] }
///     fn storage_path() -> &'static str { "avatars" }
/// }
///
/// async fn update_profile_handler(
///     avatar: OptionalFileExtractor<AvatarConfig>,
/// ) -> Result<Json<&'static str>, AppError> {
///     if let Some(file) = avatar.0 {
///         println!("Received optional avatar: {}", file.path);
///     }
///     Ok(Json("Profile updated"))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalFileExtractor<C: FileConfig>(pub Option<FileExtractor<C>>);

impl<S, C> FromRequest<S> for OptionalFileExtractor<C>
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
                None => continue,
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

            return Ok(OptionalFileExtractor(Some(FileExtractor::new(
                data,
                content_type,
                path,
                original_name,
            ))));
        }

        Ok(OptionalFileExtractor(None))
    }
}
