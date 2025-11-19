//! Core domain types for package information

use serde::{Deserialize, Serialize};

/// Registry type for package sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Registry {
    /// npm registry
    Npm,
    /// crates.io registry
    CratesIo,
    /// JSR (JavaScript Registry)
    Jsr,
}

impl Registry {
    /// Get the registry name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Registry::Npm => "npm",
            Registry::CratesIo => "crates.io",
            Registry::Jsr => "jsr",
        }
    }
}

impl std::fmt::Display for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Package information from a registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Registry source
    pub registry: Registry,
    /// Package name
    pub name: String,
    /// Latest version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Repository URL information
    pub repository: Option<RepositoryUrl>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// License identifier
    pub license: Option<String>,
}

/// Parsed repository URL information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryUrl {
    /// Repository owner/organization
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Full repository URL
    pub url: String,
}

impl RepositoryUrl {
    /// Create a new RepositoryUrl
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            url: url.into(),
        }
    }

    /// Get the GitHub API URL for this repository
    pub fn github_api_url(&self) -> String {
        format!("https://api.github.com/repos/{}/{}", self.owner, self.repo)
    }
}

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Tag name (e.g., "v1.0.0")
    pub tag_name: String,
    /// Release name/title
    pub name: Option<String>,
    /// Release description/body (markdown)
    pub body: Option<String>,
    /// Publication timestamp
    pub published_at: Option<String>,
    /// Whether this is a prerelease
    pub prerelease: bool,
    /// Whether this is a draft
    #[serde(default)]
    pub draft: bool,
}

/// Parsed changelog entry with date information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    /// Version string (e.g., "1.2.3")
    pub version: String,
    /// Release date (ISO 8601 format: "2024-01-15")
    pub date: Option<String>,
    /// Markdown content for this version
    pub content: String,
    /// Original heading line
    pub heading: String,
}

/// Parsed changelog with entries organized by version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedChangelog {
    /// Changelog entries, newest first
    pub entries: Vec<ChangelogEntry>,
    /// Unparsed content (preamble, footer, etc.)
    pub other_content: Option<String>,
}
