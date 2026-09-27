#![warn(missing_docs)]
//! # Scyph Telemetry
//!
//! Structured tracing initialization, non-blocking logging, UUIDv7 request ID middleware, response envelope metadata injection, and dynamic HTTP response compression.
//!
//! ## Key Features & Architecture
//!
//! - **Non-Blocking Tracing ([`init_tracing`])**: Offloads log writing to a dedicated worker thread via [`mod@tracing_appender::non_blocking`], preventing stdout I/O from blocking Tokio worker threads.
//! - **Panic-Safe Initialization**: Gracefully ignores double-initialization errors when running integration tests or multiple entry points.
//! - **Time-Ordered UUIDv7 Request IDs**: Generates chronologically sortable UUIDv7 request identifiers (`x-request-id`) for request tracing across log aggregators.
//! - **Axum Request ID Extractor ([`RequestId`])**: Type-safe extractor for route handlers to access the current request's UUIDv7 ID.
//! - **Span Context Injection**: Automatically attaches `request_id`, HTTP method, and URI fields to tracing `Span`s so all downstream log lines inherit request context.
//! - **Automatic Response Envelope Metadata Injection ([`auto_meta_middleware`])**: Auto-calculates request processing latency (`processing_time_ms`), attaches ISO-8601 UTC timestamps, injects request trace ID, and API version (`ResponseMeta`) directly into JSON response envelopes ([`ApiResponse`](scyph_core::ApiResponse) and [`PagedResponse`](scyph_core::PagedResponse)).
//! - **Configurable Telemetry Builder ([`TelemetryConfig`])**: Enables or disables compression, tracing, auto-meta injection, API version, and custom request ID header names.
//!
//! ## Quickstart Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use scyph_core::ApiResponse;
//! use scyph_telemetry::{init_tracing, with_telemetry, RequestId};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Initialize non-blocking tracing subscriber (keep guard in main)
//!     let _guard = init_tracing().expect("Tracing initialized");
//!
//!     // 2. Build router with handlers
//!     let app = Router::new()
//!         .route("/api/hello", get(hello_handler));
//!
//!     // 3. Attach telemetry middleware (request ID, tracing, auto-meta, compression)
//!     let app = with_telemetry(app);
//!
//!     Ok(())
//! }
//!
//! async fn hello_handler(RequestId(req_id): RequestId) -> ApiResponse<String> {
//!     ApiResponse::ok(format!("Processed request ID: {req_id}"))
//! }
//! ```

/// Tracing subscriber initialization and non-blocking appender guard.
pub mod init;

/// HTTP telemetry middleware layers, UUIDv7 request ID generation, auto-meta response injection, and Axum extractors.
pub mod middleware;

#[doc(inline)]
pub use init::{TelemetryGuard, init_tracing};
#[doc(inline)]
pub use middleware::{
    MakeRequestIdV7, RequestId, TelemetryConfig, auto_meta_middleware, with_telemetry,
    with_telemetry_config,
};
