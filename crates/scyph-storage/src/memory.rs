//! In-memory storage service adapter for unit and integration testing.

use bytes::Bytes;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{error::StorageError, traits::StorageService};

/// Stored in-memory object data containing raw bytes and MIME content-type.
#[derive(Debug, Clone)]
pub struct StoredObject {
    /// Raw byte payload of the stored object.
    pub data: Bytes,
    /// MIME content-type of the object.
    pub content_type: String,
}

/// In-memory implementation of [`StorageService`] for unit testing and local development.
///
/// Stores uploaded objects in a thread-safe `Arc<RwLock<HashMap<String, StoredObject>>>`.
/// Requires zero AWS credentials or network infrastructure, making it ideal for unit and integration tests.
///
/// # Examples
///
/// ```rust
/// use bytes::Bytes;
/// use scyph_storage::{InMemoryStorageService, StorageService};
///
/// #[tokio::main]
/// async fn main() {
///     let storage = InMemoryStorageService::new();
///
///     // Store an object
///     let path = storage
///         .store("avatars/user1.png", "image/png", Bytes::from_static(b"image data"))
///         .await
///         .unwrap();
///
///     // Retrieve the object
///     let retrieved = storage.retrieve(&path).await.unwrap();
///     assert_eq!(retrieved.as_ref(), b"image data");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryStorageService {
    objects: Arc<RwLock<HashMap<String, StoredObject>>>,
    base_url: String,
}

impl Default for InMemoryStorageService {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStorageService {
    /// Constructs a new, empty [`InMemoryStorageService`] with default base URL `"memory://storage"`.
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            base_url: "memory://storage".to_string(),
        }
    }

    /// Constructs an [`InMemoryStorageService`] with a custom base URL prefix for presigned links.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            base_url: base_url.into(),
        }
    }

    /// Returns the total number of objects currently stored in memory.
    pub fn count(&self) -> usize {
        self.objects.read().unwrap_or_else(|p| p.into_inner()).len()
    }

    /// Returns `true` if an object exists at the specified path.
    pub fn contains(&self, path: &str) -> bool {
        self.objects
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .contains_key(path)
    }

    /// Clears all objects stored in memory.
    pub fn clear(&self) {
        self.objects
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }

    fn normalize_key<'a>(&'a self, file_url: &'a str) -> &'a str {
        if let Some(stripped) = file_url.strip_prefix(&self.base_url) {
            stripped.strip_prefix('/').unwrap_or(stripped)
        } else {
            file_url
        }
    }
}

impl StorageService for InMemoryStorageService {
    async fn store(
        &self,
        path: &str,
        content_type: &str,
        data: Bytes,
    ) -> Result<String, StorageError> {
        self.objects
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .insert(
                path.to_string(),
                StoredObject {
                    data,
                    content_type: content_type.to_string(),
                },
            );
        Ok(path.to_string())
    }

    async fn retrieve(&self, file_url: &str) -> Result<Bytes, StorageError> {
        let key = self.normalize_key(file_url);

        let map = self.objects.read().unwrap_or_else(|p| p.into_inner());
        map.get(key).map(|obj| obj.data.clone()).ok_or_else(|| {
            StorageError::NotFound(format!("Object '{key}' not found in memory storage"))
        })
    }

    async fn get_view_url(
        &self,
        file_url: &str,
        _expires_in_seconds: u64,
    ) -> Result<String, StorageError> {
        let key = self.normalize_key(file_url);

        let map = self.objects.read().unwrap_or_else(|p| p.into_inner());
        if !map.contains_key(key) {
            return Err(StorageError::NotFound(format!(
                "Object '{key}' not found in memory storage"
            )));
        }

        Ok(format!("{}/{key}?view=true", self.base_url))
    }

    async fn get_view_urls(
        &self,
        file_urls: Vec<String>,
        expires_in_seconds: u64,
    ) -> Result<Vec<String>, StorageError> {
        let mut results = Vec::with_capacity(file_urls.len());
        for url in file_urls {
            results.push(self.get_view_url(&url, expires_in_seconds).await?);
        }
        Ok(results)
    }

    async fn get_download_url(
        &self,
        file_url: &str,
        download_name: &str,
        _expires_in_seconds: u64,
    ) -> Result<String, StorageError> {
        let key = self.normalize_key(file_url);
        let sanitized_name = download_name.replace(['"', '\r', '\n', '\\'], "");

        let map = self.objects.read().unwrap_or_else(|p| p.into_inner());
        if !map.contains_key(key) {
            return Err(StorageError::NotFound(format!(
                "Object '{key}' not found in memory storage"
            )));
        }

        Ok(format!("{}/{key}?download={sanitized_name}", self.base_url))
    }

    async fn delete(&self, file_url: &str) -> Result<(), StorageError> {
        let key = self.normalize_key(file_url);

        self.objects
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .remove(key);
        Ok(())
    }

    async fn test_connection(&self) -> Result<(), StorageError> {
        Ok(())
    }
}
