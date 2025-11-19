//! Error types for danny-deps

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using danny-deps Error
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in danny-deps
#[derive(Debug, Error)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// TOML parsing error
    #[error("TOML parse error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    /// Cargo.lock parsing error
    #[error("Cargo.lock parse error: {0}")]
    CargoLock(String),

    /// Invalid version requirement
    #[error("Invalid version requirement '{0}': {1}")]
    InvalidVersion(String, String),

    /// Package not found in dependency file
    #[error("Package '{0}' not found in {1}")]
    PackageNotFound(String, PathBuf),

    /// Dependency file not found
    #[error("Dependency file not found: {0}")]
    FileNotFound(PathBuf),

    /// Unsupported ecosystem
    #[error("Unsupported ecosystem: {0}")]
    UnsupportedEcosystem(String),

    /// Checksum verification failed
    #[error("Checksum verification failed for {0}: expected {1}, got {2}")]
    ChecksumMismatch(String, String, String),

    /// Invalid file format
    #[error("Invalid file format for {0}: {1}")]
    InvalidFormat(PathBuf, String),

    /// Workspace detection error
    #[error("Workspace detection error: {0}")]
    WorkspaceError(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}
