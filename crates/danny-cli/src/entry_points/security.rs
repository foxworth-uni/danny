//! Security utilities for path validation and sanitization.

use anyhow::{Context, Result};
use danny_core::Error;
use std::path::{Path, PathBuf};

/// Maximum number of files allowed in files mode to prevent resource exhaustion.
///
/// This limit prevents denial-of-service attacks where an attacker could specify
/// thousands of files (e.g., `danny **/*.js` expanding to millions of files),
/// leading to:
/// - Memory exhaustion
/// - CPU denial-of-service
/// - Filesystem handle exhaustion
///
/// The limit of 1000 files is chosen as a reasonable balance between:
/// - Allowing legitimate use cases (most file-level analysis involves < 100 files)
/// - Preventing malicious resource exhaustion
pub const MAX_FILES_IN_FILES_MODE: usize = 1000;

/// Validates that a path is within the project root (prevents path traversal)
pub fn validate_path_within_root(path: &Path, root: &Path) -> Result<()> {
    let canonical_path = path
        .canonicalize()
        .context("Failed to canonicalize path")?;
    
    let canonical_root = root
        .canonicalize()
        .context("Failed to canonicalize root")?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::PathTraversal {
            attempted_path: path.to_path_buf(),
            project_root: root.to_path_buf(),
        }
        .into());
    }

    Ok(())
}

/// Validates and sanitizes an entry point path from package.json
/// Rejects:
/// - Absolute paths
/// - Paths with glob patterns (*, ?, **)
/// - Paths outside the project root
pub fn validate_entry_point(entry_point: &str, root: &Path) -> Result<PathBuf> {
    // Reject absolute paths
    if PathBuf::from(entry_point).is_absolute() {
        return Err(Error::InvalidPath {
            path: PathBuf::from(entry_point),
            reason: "Entry point cannot be an absolute path".to_string(),
        }
        .into());
    }

    // Reject glob patterns
    if entry_point.contains('*') || entry_point.contains('?') || entry_point.contains("**") {
        return Err(Error::InvalidPath {
            path: PathBuf::from(entry_point),
            reason: "Entry point cannot contain glob patterns (*, ?, **)".to_string(),
        }
        .into());
    }

    // Join with root and validate
    let full_path = root.join(entry_point);
    
    // Validate path is within root
    validate_path_within_root(&full_path, root)?;

    // Check file exists (TOCTOU-safe: we'll open it immediately after)
    if !full_path.is_file() {
        return Err(Error::EntryPointNotFound {
            path: full_path,
        }
        .into());
    }

    Ok(full_path)
}

/// Validates multiple file paths for files mode
/// Returns error if:
/// - Too many files (DoS prevention)
/// - Any path is outside project root
pub fn validate_files_for_analysis(files: &[PathBuf], root: &Path) -> Result<()> {
    // Check resource limit
    if files.len() > MAX_FILES_IN_FILES_MODE {
        return Err(Error::TooManyFiles {
            count: files.len(),
            max_allowed: MAX_FILES_IN_FILES_MODE,
        }
        .into());
    }

    // Validate each path
    for file in files {
        validate_path_within_root(file, root)?;
    }

    Ok(())
}

