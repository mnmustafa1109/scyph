//! # Scyph Storage
//!
//! Object storage abstractions, template-driven multipart file extractors, and AWS S3/MinIO service integration for Axum applications.
//!
//! ## Overview
//!
//! `scyph-storage` provides a decoupled, type-safe architecture for handling file uploads and object storage:
//!
//! 1. **Declarative Validation Templates ([`FileConfig`])**: Zero-sized configuration structs defining field names, maximum file sizes, MIME type whitelists, and S3 directory destinations.
//! 2. **Type-Safe Axum Extractors ([`FileExtractor`] / [`MultiFileExtractor`])**: Automatic request body parsing that validates file types, streams data in chunks to prevent Out-Of-Memory (OOM) attacks, generates collision-free UUID v7 paths, and injects clean `Bytes` buffers into route handlers.
//! 3. **Unified Storage Interface ([`StorageService`])**: Async abstraction for object storage operations including streaming upload, retrieval, signed download/view URLs, deletion, and connection testing.
//! 4. **AWS S3 / MinIO Implementation ([`S3StorageService`])**: Dual-client S3 backend supporting internal cluster endpoints and browser-accessible CDN URLs.
//! 5. **Automated Health Probes ([`StorageHealthExt`])**: First-class integration with [`scyph_health::HealthRegistry`] for Kubernetes `/livez` and `/readyz` monitoring.
//!
//! ## Feature Flags
//!
//! - **`s3`**: Enables the AWS S3 SDK implementation ([`S3StorageService`]) and automatic environment loader.
//! - **`health`**: Enables [`StorageHealthExt`] integration with `scyph-health`.
//!
//! ## Quickstart Example
//!
//! ```rust,ignore
//! use axum::{extract::State, response::Json, routing::post, Router};
//! use scyph_core::error::AppError;
//! use scyph_storage::{FileConfig, FileExtractor, S3StorageService, StorageService};
//! use std::sync::Arc;
//!
//! // 1. Define a file upload template
//! pub struct AvatarUpload;
//!
//! impl FileConfig for AvatarUpload {
//!     fn field_name() -> &'static str { "avatar" }
//!     fn max_size() -> usize { 2 * 1024 * 1024 } // 2 MB
//!     fn allowed_mime_types() -> Vec<&'static str> {
//!         vec!["image/jpeg", "image/png", "image/webp"]
//!     }
//!     fn storage_path() -> &'static str { "avatars" }
//! }
//!
//! #[derive(Clone)]
//! struct AppState {
//!     storage: Arc<S3StorageService>,
//! }
//!
//! // 2. Use the extractor directly in an Axum handler
//! async fn upload_avatar_handler(
//!     State(state): State<AppState>,
//!     file: FileExtractor<AvatarUpload>,
//! ) -> Result<Json<String>, AppError> {
//!     let path = state.storage.store(&file.path, &file.content_type, file.data).await?;
//!     let view_url = state.storage.get_view_url(&path, 3600).await?;
//!     Ok(Json(view_url))
//! }
//! ```

#![warn(missing_docs)]

/// Storage error type definitions and RFC 7807 problem details conversions.
pub mod error;

/// In-memory mock storage service for testing and development.
pub mod memory;

/// Multipart form data extractors and declarative file configuration traits.
pub mod multipart;

/// Object storage service abstraction trait.
pub mod traits;

/// Health monitoring extension trait for `scyph-health` registry integration.
#[cfg(feature = "health")]
pub mod health;

/// AWS S3 and S3-compatible (MinIO, LocalStack, Cloudflare R2) storage client.
#[cfg(feature = "s3")]
pub mod s3;

#[doc(inline)]
pub use error::StorageError;

#[cfg(feature = "health")]
#[doc(inline)]
pub use health::StorageHealthExt;

#[doc(inline)]
pub use memory::InMemoryStorageService;

#[doc(inline)]
pub use multipart::{
    ExtractedFile, FileConfig, FileExtractor, MultiFileExtractor, OptionalFileExtractor,
};

#[doc(inline)]
pub use traits::StorageService;

#[cfg(feature = "s3")]
#[doc(inline)]
pub use s3::S3StorageService;
