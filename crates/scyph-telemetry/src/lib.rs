#![warn(missing_docs)]
//! # Scyph Telemetry
//!
//! Structured tracing initialization, non-blocking logging, UUIDv7 request ID middleware, and dynamic HTTP response compression.
//!
//! ## Key Features & Architecture
//!
//! - **Non-Blocking Tracing ([`init_tracing`])**: Offloads log writing to a dedicated worker thread via [`mod@tracing_appender::non_blocking`], preventing stdout I/O from blocking Tokio worker threads.
//! - **Panic-Safe Initialization**: Gracefully ignores double-initialization errors when running integration tests or multiple entry points.
//! - **Time-Ordered UUIDv7 Request IDs**: Generates chronologically sortable UUIDv7 request identifiers (`x-request-id`) for request tracing across log aggregators.
//! - **Axum Request ID Extractor ([`RequestId`])**: Type-safe extractor for route handlers to access the current request's UUIDv7 ID.
//! - **Span Context Injection**: Automatically attaches `request_id`, HTTP method, and URI fields to tracing `Span`s so all downstream log lines inherit request context.
//! - **Configurable Telemetry Builder ([`TelemetryConfig`])**: Enables or disables compression, tracing, and custom request ID header names.
//!
//! ## Quickstart Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph_telemetry::{init_tracing, with_telemetry, RequestId};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Initialize tracing subscriber (keep guard in main)
//!     let _guard = init_tracing().expect("Tracing initialized");
//!
//!     // 2. Build router with telemetry middleware
//!     let app = Router::new()
//!         .route("/api/hello", get(hello_handler));
//!
//!     let app = with_telemetry(app);
//!
//!     Ok(())
//! }
//!
//! async fn hello_handler(RequestId(req_id): RequestId) -> String {
//!     format!("Processed request ID: {req_id}")
//! }
//! ```

/// Tracing subscriber initialization and non-blocking appender guard.
pub mod init;

/// HTTP telemetry middleware layers, UUIDv7 request ID generation, and Axum extractors.
pub mod middleware;

#[doc(inline)]
pub use init::{init_tracing, TelemetryGuard};
#[doc(inline)]
pub use middleware::{
    auto_meta_middleware, with_telemetry, with_telemetry_config, MakeRequestIdV7, RequestId,
    TelemetryConfig,
};
