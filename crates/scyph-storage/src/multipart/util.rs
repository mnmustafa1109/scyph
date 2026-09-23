//! Internal stream processing utilities for multipart extractors.

use axum::extract::multipart::Field;
use bytes::{Bytes, BytesMut};

use crate::error::StorageError;

/// Safely buffers an incoming multipart field stream in chunks, immediately aborting
/// if the accumulated byte count exceeds `max_size` to prevent Out-of-Memory (OOM) attacks.
pub(crate) async fn read_field_bytes(
    mut field: Field<'_>,
    max_size: usize,
    field_label: &str,
) -> Result<Bytes, StorageError> {
    let mut buffer = BytesMut::new();
    while let Some(chunk) = field.chunk().await.map_err(|e| {
        StorageError::InvalidFile(format!("Error reading field '{field_label}' chunk: {e}"))
    })? {
        if buffer.len() + chunk.len() > max_size {
            return Err(StorageError::FileTooLarge(format!(
                "File '{field_label}' exceeds maximum allowed size of {max_size} bytes"
            )));
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(buffer.freeze())
}
