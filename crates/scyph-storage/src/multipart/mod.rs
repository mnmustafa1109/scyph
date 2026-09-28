//! Multipart form data extractors and declarative file configuration traits.
//!
//! This module provides the building blocks for type-safe, validated file upload handling
//! in Axum web applications. The core types are:
//!
//! - [`FileConfig`]: A trait implemented on zero-sized structs defining upload constraints
//! - [`FileExtractor<C>`]: Axum extractor for a single required file upload
//! - [`MultiFileExtractor<C>`]: Axum extractor for multiple files in one request
//! - [`OptionalFileExtractor<C>`]: Axum extractor for an optional single file
//! - [`ExtractedFile`]: Individual validated file from a multi-file upload

pub mod config;
pub mod multi;
pub mod optional;
pub mod single;
pub(crate) mod util;

#[doc(inline)]
pub use config::FileConfig;

#[doc(inline)]
pub use multi::{ExtractedFile, MultiFileExtractor};

#[doc(inline)]
pub use optional::OptionalFileExtractor;

#[doc(inline)]
pub use single::FileExtractor;
