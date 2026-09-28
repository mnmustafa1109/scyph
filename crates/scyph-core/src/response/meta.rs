//! Standardized metadata block attached to API response envelopes.
//!
//! This module provides [`ResponseMeta`], an optional metadata header that can be attached to
//! any [`ApiResponse`](super::ApiResponse) or [`PagedResponse`](super::PagedResponse) to surface
//! request-level diagnostics to API clients.
//!
//! # Purpose
//!
//! Attaching `ResponseMeta` to responses enables clients to:
//! - Correlate logs and traces using the `trace_id` field (e.g., for support tickets).
//! - Monitor latency from the server's perspective via `processing_time_ms`.
//! - Detect API version mismatches between clients and servers.
//!
//! # Wire Format
//!
//! When included in a response, `ResponseMeta` serializes as:
//!
//! ```json
//! {
//!   "meta": {
//!     "processing_time_ms": 8,
//!     "timestamp": "2026-09-28T12:00:00.000000000+00:00",
//!     "trace_id": "01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9",
//!     "version": "0.1.0"
//!   }
//! }
//! ```
//!
//! # Integration Pattern
//!
//! Typically, `ResponseMeta` is constructed in a middleware or request lifecycle hook that
//! records the start time before passing the request to handlers, then attaches metadata to
//! the response on the way out:
//!
//! ```rust
//! use scyph_core::{ApiResponse, ResponseMeta};
//! use std::time::Instant;
//!
//! fn build_response_with_meta(data: &'static str, trace_id: &str) -> ApiResponse<&'static str> {
//!     let start = Instant::now();
//!     // ... process request ...
//!     let elapsed_ms = start.elapsed().as_millis() as u64;
//!     let meta = ResponseMeta::new(trace_id, elapsed_ms, "0.1.0");
//!     ApiResponse::ok(data).with_meta(meta)
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Standardized metadata header attached to API response envelopes.
///
/// Contains execution timing, request trace ID, timestamp, and application API version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Request processing latency in milliseconds.
    pub processing_time_ms: u64,
    /// ISO-8601 UTC timestamp of response generation.
    pub timestamp: String,
    /// Unique UUIDv7 request trace ID associated with the HTTP request.
    pub trace_id: String,
    /// API application version string.
    pub version: String,
}

impl ResponseMeta {
    /// Creates a new [`ResponseMeta`] instance with current UTC timestamp and given parameters.
    ///
    /// # Arguments
    ///
    /// * `trace_id` - Unique trace or request identifier.
    /// * `processing_time_ms` - Elapsed duration in milliseconds for handling the request.
    /// * `version` - API version string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_core::ResponseMeta;
    ///
    /// let meta = ResponseMeta::new("01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9", 12, "0.1.0");
    /// assert_eq!(meta.processing_time_ms, 12);
    /// assert_eq!(meta.version, "0.1.0");
    /// assert_eq!(meta.trace_id, "01923f81-5c8e-7e9b-b4a1-8d2f1e4067a9");
    /// ```
    pub fn new(
        trace_id: impl Into<String>,
        processing_time_ms: u64,
        version: impl Into<String>,
    ) -> Self {
        Self {
            processing_time_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.into(),
            version: version.into(),
        }
    }
}
