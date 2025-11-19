//! Core traits for dependency management

use crate::types::{DependencyFile, DependencyUpdate, UpdateResult};
use crate::Result;
use danny_fs::FileSystem;
use std::path::Path;
use std::sync::Arc;

/// Main trait for dependency file management
///
/// This trait provides an abstraction over different dependency file formats
/// (Cargo.toml, package.json, pyproject.toml, etc.).
#[async_trait::async_trait]
pub trait DependencyManager: Send + Sync {
    /// Parse a dependency file from the given path
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    async fn parse<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<DependencyFile>;

    /// Apply updates to a dependency file
    ///
    /// This method preserves formatting and comments where possible.
    ///
    /// # Arguments
    /// * `fs` - FileSystem instance for file operations
    /// * `path` - Path to the dependency file
    /// * `updates` - List of updates to apply
    /// * `dry_run` - If true, don't actually write changes
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, parsed, or written
    async fn update<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
        updates: &[DependencyUpdate],
        dry_run: bool,
    ) -> Result<UpdateResult>;

    /// Validate a dependency file
    ///
    /// Checks for common issues like missing fields, invalid versions, etc.
    ///
    /// # Errors
    /// Returns an error if validation fails
    async fn validate<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<()>;

    /// Detect if the given path is a workspace root
    async fn is_workspace_root<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<bool>;

    /// Find all workspace members starting from a root
    async fn find_workspace_members<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        root: &Path,
    ) -> Result<Vec<std::path::PathBuf>>;
}

/// Trait for parsing lockfiles
#[async_trait::async_trait]
pub trait LockfileParser: Send + Sync {
    /// Parse a lockfile and extract installed versions
    ///
    /// # Errors
    /// Returns an error if the lockfile cannot be read or parsed
    async fn parse_lockfile<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
    ) -> Result<LockedDependencies>;

    /// Verify lockfile integrity (checksums)
    ///
    /// # Errors
    /// Returns an error if checksum verification fails
    async fn verify_integrity<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<()>;
}

/// Represents the locked/installed versions from a lockfile
#[derive(Debug, Clone)]
pub struct LockedDependencies {
    /// Map of package name to installed version
    pub packages: std::collections::HashMap<String, LockedPackage>,
}

/// Information about a locked package
#[derive(Debug, Clone)]
pub struct LockedPackage {
    /// Package name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Checksum/integrity hash
    pub checksum: Option<String>,
    /// Resolved URL or source
    pub resolved: Option<String>,
}
