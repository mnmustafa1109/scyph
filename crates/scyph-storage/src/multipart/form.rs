//! Combined multipart form data and file upload extractors.

use std::marker::PhantomData;

use axum::{
    extract::{FromRequest, Multipart, Request},
};
use scyph_core::error::AppError;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    error::StorageError,
    multipart::{
        config::FileConfig,
        multi::ExtractedFile,
        single::FileExtractor,
        util::read_field_bytes,
    },
};

/// Axum extractor for combined form data fields and a single file upload.
///
/// Automatically parses the multipart boundary, extracts text form fields into `F`,
/// validates the file against [`FileConfig`], and enforces payload size limits.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::response::Json;
/// use scyph_core::error::AppError;
/// use scyph_storage::{FileConfig, FormDataWithFile};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// pub struct CreateUserProfile {
///     pub username: String,
///     pub bio: Option<String>,
/// }
///
/// pub struct AvatarConfig;
/// impl FileConfig for AvatarConfig {
///     fn field_name() -> &'static str { "avatar" }
///     fn max_size() -> usize { 2 * 1024 * 1024 }
///     fn allowed_mime_types() -> Vec<&'static str> { vec!["image/png", "image/jpeg"] }
///     fn storage_path() -> &'static str { "avatars" }
/// }
///
/// async fn handle_upload(
///     FormDataWithFile { data, file }: FormDataWithFile<CreateUserProfile, AvatarConfig>,
/// ) -> Result<Json<String>, AppError> {
///     Ok(Json(format!("User {} uploaded {}", data.username, file.path)))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FormDataWithFile<F, C: FileConfig> {
    /// Deserialized text form fields.
    pub data: F,
    /// Extracted and validated file payload.
    pub file: FileExtractor<C>,
}

impl<S, F, C> FromRequest<S> for FormDataWithFile<F, C>
where
    S: Send + Sync,
    F: DeserializeOwned + Send + 'static,
    C: FileConfig,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart read error: {e}")))?;

        let mut form_fields = serde_json::Map::new();
        let mut extracted_file: Option<FileExtractor<C>> = None;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let name = field.name().unwrap_or_default().to_string();

            if name == C::field_name() && extracted_file.is_none() {
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

                let clean_mime = content_type
                    .split(';')
                    .next()
                    .unwrap_or(&content_type)
                    .trim()
                    .to_ascii_lowercase();

                let allowed = C::allowed_mime_types();
                let matches_mime = allowed.iter().any(|m| m.eq_ignore_ascii_case(&clean_mime));

                if !matches_mime {
                    return Err(StorageError::UnsupportedMediaType(format!(
                        "Unsupported Content-Type '{content_type}' for field '{}'. Allowed MIME types: {:?}",
                        C::field_name(),
                        allowed
                    ))
                    .into());
                }

                let data = read_field_bytes(field, C::max_size(), &original_name).await?;

                if C::enforce_magic_bytes() && !C::verify_magic_bytes(&data, &clean_mime) {
                    return Err(StorageError::UnsupportedMediaType(format!(
                        "File '{original_name}' content signature does not match declared MIME type '{content_type}'"
                    ))
                    .into());
                }

                let extension = C::resolve_extension(&clean_mime);
                let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

                extracted_file = Some(FileExtractor::new(data, content_type, path, original_name));
            } else if field.file_name().is_none() {
                let text = field
                    .text()
                    .await
                    .map_err(|e| StorageError::InvalidFile(format!("Failed to read text field '{name}': {e}")))?;

                if (name == "data" || name == "payload") && text.trim_start().starts_with('{') {
                    if let Ok(serde_json::Value::Object(nested_map)) = serde_json::from_str(&text) {
                        for (k, v) in nested_map {
                            form_fields.insert(k, v);
                        }
                        continue;
                    }
                }

                if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(&text) {
                    form_fields.insert(name, parsed_json);
                } else {
                    form_fields.insert(name, serde_json::Value::String(text));
                }
            }
        }

        let file = extracted_file.ok_or_else(|| {
            StorageError::InvalidFile(format!(
                "Required file field '{}' was not found in request",
                C::field_name()
            ))
        })?;

        let data = serde_json::from_value(serde_json::Value::Object(form_fields))
            .map_err(|e| AppError::BadRequest(format!("Invalid form field data: {e}")))?;

        Ok(FormDataWithFile { data, file })
    }
}

/// Axum extractor for combined form data fields and an optional file upload.
#[derive(Debug, Clone)]
pub struct FormDataWithOptionalFile<F, C: FileConfig> {
    /// Deserialized text form fields.
    pub data: F,
    /// Optional extracted file payload.
    pub file: Option<FileExtractor<C>>,
}

impl<S, F, C> FromRequest<S> for FormDataWithOptionalFile<F, C>
where
    S: Send + Sync,
    F: DeserializeOwned + Send + 'static,
    C: FileConfig,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart read error: {e}")))?;

        let mut form_fields = serde_json::Map::new();
        let mut extracted_file: Option<FileExtractor<C>> = None;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let name = field.name().unwrap_or_default().to_string();

            if name == C::field_name() && extracted_file.is_none() {
                if let Some(file_name) = field.file_name() && !file_name.trim().is_empty() {
                    let original_name = file_name.to_string();
                    let content_type = field
                        .content_type()
                        .map(|ct| ct.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string());

                    let clean_mime = content_type
                        .split(';')
                        .next()
                        .unwrap_or(&content_type)
                        .trim()
                        .to_ascii_lowercase();

                    let allowed = C::allowed_mime_types();
                    let matches_mime = allowed.iter().any(|m| m.eq_ignore_ascii_case(&clean_mime));

                    if !matches_mime {
                        return Err(StorageError::UnsupportedMediaType(format!(
                            "Unsupported Content-Type '{content_type}' for field '{}'",
                            C::field_name()
                        ))
                        .into());
                    }

                    let data = read_field_bytes(field, C::max_size(), &original_name).await?;

                    if C::enforce_magic_bytes() && !C::verify_magic_bytes(&data, &clean_mime) {
                        return Err(StorageError::UnsupportedMediaType(format!(
                            "File '{original_name}' content signature does not match declared MIME type"
                        ))
                        .into());
                    }

                    let extension = C::resolve_extension(&clean_mime);
                    let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

                    extracted_file = Some(FileExtractor::new(data, content_type, path, original_name));
                }
            } else if field.file_name().is_none() {
                let text = field
                    .text()
                    .await
                    .map_err(|e| StorageError::InvalidFile(format!("Failed to read text field '{name}': {e}")))?;

                if (name == "data" || name == "payload") && text.trim_start().starts_with('{') {
                    if let Ok(serde_json::Value::Object(nested_map)) = serde_json::from_str(&text) {
                        for (k, v) in nested_map {
                            form_fields.insert(k, v);
                        }
                        continue;
                    }
                }

                if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(&text) {
                    form_fields.insert(name, parsed_json);
                } else {
                    form_fields.insert(name, serde_json::Value::String(text));
                }
            }
        }

        let data = serde_json::from_value(serde_json::Value::Object(form_fields))
            .map_err(|e| AppError::BadRequest(format!("Invalid form field data: {e}")))?;

        Ok(FormDataWithOptionalFile {
            data,
            file: extracted_file,
        })
    }
}

/// Axum extractor for combined form data fields and multiple file uploads.
#[derive(Debug, Clone)]
pub struct FormDataWithFiles<F, C: FileConfig> {
    /// Deserialized text form fields.
    pub data: F,
    /// Extracted and validated file payloads.
    pub files: Vec<ExtractedFile>,
    _config: PhantomData<C>,
}

impl<S, F, C> FromRequest<S> for FormDataWithFiles<F, C>
where
    S: Send + Sync,
    F: DeserializeOwned + Send + 'static,
    C: FileConfig,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart read error: {e}")))?;

        let mut form_fields = serde_json::Map::new();
        let mut extracted_files = Vec::new();

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::InvalidFile(format!("Multipart field read error: {e}")))?
        {
            let name = field.name().unwrap_or_default().to_string();

            if name == C::field_name() {
                if extracted_files.len() >= C::max_files() {
                    return Err(StorageError::InvalidFile(format!(
                        "Maximum file upload count ({}) exceeded for field '{}'",
                        C::max_files(),
                        C::field_name()
                    ))
                    .into());
                }

                let original_name = match field.file_name() {
                    Some(name) if !name.trim().is_empty() => name.to_string(),
                    _ => continue,
                };

                let content_type = field
                    .content_type()
                    .map(|ct| ct.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                let clean_mime = content_type
                    .split(';')
                    .next()
                    .unwrap_or(&content_type)
                    .trim()
                    .to_ascii_lowercase();

                let allowed = C::allowed_mime_types();
                let matches_mime = allowed.iter().any(|m| m.eq_ignore_ascii_case(&clean_mime));

                if !matches_mime {
                    return Err(StorageError::UnsupportedMediaType(format!(
                        "Unsupported Content-Type '{content_type}' for field '{}'",
                        C::field_name()
                    ))
                    .into());
                }

                let data = read_field_bytes(field, C::max_size(), &original_name).await?;

                if C::enforce_magic_bytes() && !C::verify_magic_bytes(&data, &clean_mime) {
                    return Err(StorageError::UnsupportedMediaType(format!(
                        "File '{original_name}' content signature does not match declared MIME type"
                    ))
                    .into());
                }

                let extension = C::resolve_extension(&clean_mime);
                let path = format!("{}/{}.{}", C::storage_path(), Uuid::now_v7(), extension);

                extracted_files.push(ExtractedFile {
                    data,
                    content_type,
                    path,
                    original_name,
                });
            } else if field.file_name().is_none() {
                let text = field
                    .text()
                    .await
                    .map_err(|e| StorageError::InvalidFile(format!("Failed to read text field '{name}': {e}")))?;

                if (name == "data" || name == "payload") && text.trim_start().starts_with('{') {
                    if let Ok(serde_json::Value::Object(nested_map)) = serde_json::from_str(&text) {
                        for (k, v) in nested_map {
                            form_fields.insert(k, v);
                        }
                        continue;
                    }
                }

                if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(&text) {
                    form_fields.insert(name, parsed_json);
                } else {
                    form_fields.insert(name, serde_json::Value::String(text));
                }
            }
        }

        let data = serde_json::from_value(serde_json::Value::Object(form_fields))
            .map_err(|e| AppError::BadRequest(format!("Invalid form field data: {e}")))?;

        Ok(FormDataWithFiles {
            data,
            files: extracted_files,
            _config: PhantomData,
        })
    }
}
