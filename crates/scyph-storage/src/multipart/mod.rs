//! Multipart form data extractors and declarative file configuration traits.

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
