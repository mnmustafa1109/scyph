//! Image thumbnail generation, resizing, format conversion, and key derivation.
//!
//! Provides a complete pipeline for generating thumbnails from uploaded images:
//!
//! ## Components
//!
//! - **[`ThumbnailConfig`]** (`config`): Configures thumbnail dimensions, output format (JPEG/PNG/WebP),
//!   JPEG quality, and decompression bomb limits. The primary type for thumbnail generation.
//! - **Generator** (`generator`): Low-level image decoding, aspect-ratio-preserving resize, and
//!   re-encoding functions exposed through `ThumbnailConfig::generate`.
//! - **Key Derivation** (`key`): Helpers for deriving S3 storage key paths for thumbnails
//!   (e.g., `"avatars/abc.png"` → `"thumbnails/avatars/abc.webp"`).
//! - **[`StorageThumbnailExt`]** (`service_ext`): Extension trait on [`crate::traits::StorageService`]
//!   providing `store_with_thumbnail` for atomic dual-upload in one call.
//!
//! ## Decompression Bomb Protection
//!
//! Thumbnail decoding runs on `tokio::task::spawn_blocking` and enforces hard limits via
//! `image::Limits` — a maximum decoded buffer allocation of 64 MiB (`MAX_DECODE_BYTES`).
//! Oversized or maliciously crafted images are rejected before pixel data can exhaust memory.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use scyph_storage::{StorageThumbnailExt, ThumbnailConfig};
//!
//! let config = ThumbnailConfig::new(200, 200);
//!
//! // Upload original + thumbnail in one call:
//! let result = storage.store_with_thumbnail(
//!     "avatars/user_123.png",
//!     "image/png",
//!     file_bytes,
//!     config,
//! ).await?;
//!
//! println!("Original: {}", result.original_key);    // "avatars/user_123.png"
//! println!("Thumbnail: {}", result.thumbnail_key);  // "thumbnails/avatars/user_123.png"
//! ```

pub mod config;
pub mod generator;
pub mod key;
pub mod service_ext;

pub use config::*;
pub use generator::*;
pub use key::*;
pub use service_ext::*;
