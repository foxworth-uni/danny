//! Error types for fob-info

use thiserror::Error;

/// Result type alias for fob-info operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for fob-info operations
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON deserialization failed
    #[error("Failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid package name format
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    /// Invalid URL format
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Package not found in registry
    #[error("Package '{0}' not found in {1} registry")]
    PackageNotFound(String, String),

    /// Repository information not available
    #[error("Repository information not available for package '{0}'")]
    RepositoryNotAvailable(String),

    /// GitHub API error
    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    /// Changelog not found
    #[error("Changelog not found for repository {0}/{1}")]
    ChangelogNotFound(String, String),

    /// Invalid repository URL format
    #[error("Invalid repository URL format: {0}")]
    InvalidRepositoryUrl(String),

    /// Unsupported repository host
    #[error("Unsupported repository host: {0} (only GitHub is supported)")]
    UnsupportedRepositoryHost(String),

    /// Rate limit exceeded (HTTP 429)
    #[error("Rate limit exceeded for URL: {0}")]
    RateLimitExceeded(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
