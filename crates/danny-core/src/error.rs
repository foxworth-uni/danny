//! Error types for Danny core.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for Danny operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during analysis.
#[derive(Debug, Error)]
pub enum Error {
    /// An entry point file does not exist.
    #[error("Entry point not found: {path}")]
    EntryPointNotFound {
        /// Path to the missing entry point.
        path: PathBuf,
    },

    /// Configuration file is invalid.
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the configuration error.
        message: String,
    },

    /// TOML parsing error.
    #[error("TOML parse error in {file}: {source}")]
    TomlError {
        /// Path to the TOML file with the error.
        file: PathBuf,
        /// The underlying TOML parsing error.
        #[source]
        source: toml::de::Error,
    },

    /// JSON parsing error (package.json, etc.).
    #[error("JSON parse error in {file}: {source}")]
    JsonError {
        /// Path to the JSON file with the error.
        file: PathBuf,
        /// The underlying JSON parsing error.
        #[source]
        source: serde_json::Error,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Backend-specific error.
    #[error("Backend error ({backend}): {message}")]
    Backend {
        /// Name of the backend that encountered the error.
        backend: String,
        /// Error message from the backend.
        message: String,
    },

    /// Pattern matching error.
    #[error("Pattern error: {0}")]
    Pattern(String),

    /// No suitable backend found.
    #[error("No backend found for file extension: {extension}")]
    NoBackendForExtension {
        /// The file extension that no backend supports.
        extension: String,
    },

    /// Multiple backends found (ambiguous).
    #[error("Multiple backends found for extension {extension}: {backends:?}")]
    AmbiguousBackend {
        /// The file extension with multiple handlers.
        extension: String,
        /// Names of the backends that claim to support this extension.
        backends: Vec<String>,
    },

    /// Graph too large for analysis
    #[error("Graph has {module_count} modules, max allowed is {max_allowed}")]
    GraphTooLarge {
        module_count: usize,
        max_allowed: usize,
    },

    /// Circular dependency cycle too deep
    #[error("Circular dependency depth {depth} exceeds max {max_allowed}")]
    CycleTooDeep {
        depth: usize,
        max_allowed: usize,
    },

    /// Path traversal attempt detected
    #[error("Path traversal: {attempted_path:?} outside {project_root:?}")]
    PathTraversal {
        attempted_path: PathBuf,
        project_root: PathBuf,
    },

    /// Invalid path
    #[error("Invalid path {path:?}: {reason}")]
    InvalidPath {
        path: PathBuf,
        reason: String,
    },

    /// User cancelled the operation
    #[error("User cancelled operation")]
    UserCancelled,

    /// No package.json found
    #[error("No package.json found in {searched}")]
    NoPackageJson {
        /// Path that was searched
        searched: PathBuf,
    },

    /// Too many files specified for files mode
    #[error("Too many files specified: {count} files exceeds maximum of {max_allowed}")]
    TooManyFiles {
        /// Number of files specified
        count: usize,
        /// Maximum allowed
        max_allowed: usize,
    },
}
