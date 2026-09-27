//! Image thumbnail generation utilities for storage pipelines.
//!
//! Provides configurable image decoding, aspect-ratio-preserving resizing,
//! format encoding, storage path key derivation, and storage service upload extensions.

pub mod config;
pub mod generator;
pub mod key;
pub mod service_ext;

pub use config::*;
pub use generator::*;
pub use key::*;
pub use service_ext::*;
