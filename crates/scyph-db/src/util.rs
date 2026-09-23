//! Internal utility functions for `scyph-db`.
//!
//! Provides filesystem inspection and environment resolution helpers for migration and seed runner routines.

use std::path::{Path, PathBuf};

/// Checks if a directory contains any `.sql` files directly within it.
///
/// Iterates over directory entries in `dir` and checks if any regular file possesses a `.sql` extension.
/// Subdirectories are not traversed.
///
/// # Arguments
///
/// * `dir` - Path reference to the directory being inspected.
///
/// # Returns
///
/// Returns `true` if at least one `.sql` file exists directly within `dir`, `false` otherwise (or if reading the directory fails).
pub(crate) fn has_sql_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        entries.filter_map(Result::ok).any(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && e.path().extension().and_then(|ext| ext.to_str()) == Some("sql")
        })
    } else {
        false
    }
}

/// Resolves environment string from `APP_ENV` to standard target folder name.
///
/// Canonical mappings:
/// - `"development"` or `"dev"` (or default when unset) maps to `"beta"`.
/// - `"production"` or `"prod"` maps to `"prod"`.
/// - Any other custom string (e.g. `"staging"`) is preserved as-is.
///
/// # Arguments
///
/// * `raw_env` - Raw environment identifier string (e.g. from `APP_ENV`).
///
/// # Returns
///
/// Returns the normalized target folder name string.
pub(crate) fn resolve_env_folder(raw_env: &str) -> String {
    match raw_env.to_lowercase().as_str() {
        "development" | "dev" => "beta".to_string(),
        "production" => "prod".to_string(),
        _ => raw_env.to_string(),
    }
}

/// Resolves the actual existing environment folder path inside `base_dir`.
///
/// Evaluates the mapped folder name (e.g., `beta` or `prod`) first, and falls back to `raw_env` if distinct.
///
/// # Arguments
///
/// * `base_dir` - Base root directory path (e.g. `./migrations` or `./seeds`).
/// * `raw_env` - Environment identifier string.
///
/// # Returns
///
/// Returns `Some(PathBuf)` pointing to the existing environment directory, or `None` if no matching directory exists.
pub(crate) fn resolve_target_env_path(base_dir: &Path, raw_env: &str) -> Option<PathBuf> {
    let mapped_name = resolve_env_folder(raw_env);
    let mapped_path = base_dir.join(&mapped_name);
    if mapped_path.exists() {
        return Some(mapped_path);
    }
    let raw_path = base_dir.join(raw_env);
    if raw_path.exists() {
        return Some(raw_path);
    }
    None
}
