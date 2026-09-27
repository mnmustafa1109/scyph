//! Standardized metadata block attached to API response envelopes.

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
