//! Package metadata fetcher for npm, crates.io, JSR, and GitHub
//!
//! This library provides a simple API to fetch package information from multiple
//! package registries and GitHub repositories.
//!
//! # Example
//!
//! ```no_run
//! use danny_info::InfoClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = InfoClient::new()?;
//!
//!     // Fetch npm package
//!     let react = client.fetch_npm("react").await?;
//!     println!("React v{}", react.version);
//!
//!     // Fetch Rust crate
//!     let serde = client.fetch_crates_io("serde").await?;
//!     println!("Serde v{}", serde.version);
//!
//!     // Fetch GitHub releases if repository is available
//!     if let Some(repo) = react.repository {
//!         let releases = client.fetch_releases(&repo).await?;
//!         println!("Latest release: {}", releases[0].tag_name);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod changelog_parser;
mod client;
mod crates_io;
mod error;
mod github;
mod jsr;
mod npm;
mod repository;
mod types;

pub use error::{Error, Result};
pub use types::{ChangelogEntry, PackageInfo, ParsedChangelog, Registry, Release, RepositoryUrl};

use client::HttpClient;

/// Main client for fetching package information
///
/// This is the primary interface for all package registry and GitHub operations.
///
/// By default, rate limiting is enabled to comply with registry requirements:
/// - npm: 1 request/second (conservative)
/// - crates.io: 1 request/second (required)
/// - JSR: 1 request/second (conservative)
/// - GitHub: 60 requests/hour unauthenticated, 5000 requests/hour authenticated
pub struct InfoClient {
    npm_client: HttpClient,
    crates_io_client: HttpClient,
    jsr_client: HttpClient,
    github_client: HttpClient,
}

impl InfoClient {
    /// Create a new InfoClient with rate limiting enabled (recommended)
    ///
    /// Rate limits:
    /// - npm, crates.io, JSR: 1 request/second
    /// - GitHub: Handled by GitHub API (60/hr unauthenticated, 5000/hr with GITHUB_TOKEN)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP clients cannot be initialized.
    pub fn new() -> Result<Self> {
        Ok(Self {
            npm_client: HttpClient::with_rate_limit(1)?,
            crates_io_client: HttpClient::with_rate_limit(1)?,
            jsr_client: HttpClient::with_rate_limit(1)?,
            github_client: HttpClient::new()?, // GitHub enforces its own rate limits
        })
    }

    /// Create a new InfoClient without rate limiting (use with caution!)
    ///
    /// This disables client-side rate limiting. Use only when:
    /// - You have explicit permission for higher rates
    /// - You are handling rate limiting yourself
    /// - You are testing against a local/mock server
    ///
    /// Note: crates.io requires 1 req/sec maximum. Using this method may result
    /// in your requests being blocked.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP clients cannot be initialized.
    pub fn without_rate_limiting() -> Result<Self> {
        Ok(Self {
            npm_client: HttpClient::new()?,
            crates_io_client: HttpClient::new()?,
            jsr_client: HttpClient::new()?,
            github_client: HttpClient::new()?,
        })
    }

    /// Create a new InfoClient with a custom user agent and rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP clients cannot be initialized.
    pub fn with_user_agent(_user_agent: impl Into<String>) -> Result<Self> {
        // Note: HttpClient::with_rate_limit currently doesn't support custom user agent
        // This is a limitation we can address if needed
        // For now, we use the default user agent with rate limiting
        Ok(Self {
            npm_client: HttpClient::with_rate_limit(1)?,
            crates_io_client: HttpClient::with_rate_limit(1)?,
            jsr_client: HttpClient::with_rate_limit(1)?,
            github_client: HttpClient::new()?,
        })
    }

    /// Fetch package information from npm registry
    ///
    /// # Arguments
    ///
    /// * `name` - Package name (e.g., "react" or "@types/node")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::InfoClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let info = client.fetch_npm("react").await?;
    /// println!("React v{}", info.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_npm(&self, name: &str) -> Result<PackageInfo> {
        npm::fetch_npm_package(&self.npm_client, name).await
    }

    /// Fetch package information from crates.io registry
    ///
    /// # Arguments
    ///
    /// * `name` - Crate name (e.g., "serde")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::InfoClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let info = client.fetch_crates_io("serde").await?;
    /// println!("Serde v{}", info.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_crates_io(&self, name: &str) -> Result<PackageInfo> {
        crates_io::fetch_crates_io_package(&self.crates_io_client, name).await
    }

    /// Fetch package information from JSR (JavaScript Registry)
    ///
    /// # Arguments
    ///
    /// * `name` - Package name in format "@scope/package" (e.g., "@std/path")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::InfoClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let info = client.fetch_jsr("@std/path").await?;
    /// println!("@std/path v{}", info.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_jsr(&self, name: &str) -> Result<PackageInfo> {
        jsr::fetch_jsr_package(&self.jsr_client, name).await
    }

    /// Fetch releases from a GitHub repository
    ///
    /// Returns up to 100 releases (GitHub API default limit).
    /// Set GITHUB_TOKEN environment variable for higher rate limits.
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository URL information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::{InfoClient, RepositoryUrl};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let repo = RepositoryUrl::new("facebook", "react", "https://github.com/facebook/react");
    /// let releases = client.fetch_releases(&repo).await?;
    /// println!("Latest: {}", releases[0].tag_name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_releases(&self, repo: &RepositoryUrl) -> Result<Vec<Release>> {
        github::fetch_releases(&self.github_client, repo).await
    }

    /// Fetch changelog content from a GitHub repository
    ///
    /// Searches for common changelog filenames (CHANGELOG.md, CHANGES.md, etc.)
    /// and returns the raw markdown content of the first found file.
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository URL information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::{InfoClient, RepositoryUrl};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let repo = RepositoryUrl::new("facebook", "react", "https://github.com/facebook/react");
    /// let changelog = client.fetch_changelog(&repo).await?;
    /// println!("Changelog: {} bytes", changelog.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_changelog(&self, repo: &RepositoryUrl) -> Result<String> {
        github::fetch_changelog(&self.github_client, repo).await
    }

    /// Fetch and parse a changelog with date extraction
    ///
    /// Returns structured changelog entries with version numbers and dates.
    /// This is more useful than `fetch_changelog()` if you need to work with
    /// specific versions or time ranges.
    ///
    /// Supports common changelog formats:
    /// - Keep a Changelog: `## [1.2.3] - 2024-01-15`
    /// - Angular style: `## 1.2.3 (2024-01-15)`
    /// - Timestamp format: `# v1.2.3 / 2024-01-15`
    /// - Version only: `## [1.2.3]` or `## 1.2.3`
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository URL information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use danny_info::{InfoClient, RepositoryUrl};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = InfoClient::new()?;
    /// let repo = RepositoryUrl::new("facebook", "react", "https://github.com/facebook/react");
    /// let parsed = client.fetch_parsed_changelog(&repo).await?;
    ///
    /// for entry in parsed.entries {
    ///     println!("{} - {}", entry.version, entry.date.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_parsed_changelog(&self, repo: &RepositoryUrl) -> Result<ParsedChangelog> {
        let markdown = github::fetch_changelog(&self.github_client, repo).await?;
        Ok(changelog_parser::parse_changelog(&markdown))
    }

    /// Parse a repository URL string into a RepositoryUrl
    ///
    /// Supports various GitHub URL formats:
    /// - https://github.com/owner/repo
    /// - https://github.com/owner/repo.git
    /// - git+https://github.com/owner/repo.git
    /// - git@github.com:owner/repo.git
    ///
    /// # Example
    ///
    /// ```
    /// # use danny_info::InfoClient;
    /// let repo = InfoClient::parse_repository_url("https://github.com/facebook/react").unwrap();
    /// assert_eq!(repo.owner, "facebook");
    /// assert_eq!(repo.repo, "react");
    /// ```
    pub fn parse_repository_url(url: &str) -> Result<RepositoryUrl> {
        repository::parse_repository_url(url)
    }
}

impl Default for InfoClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default InfoClient")
    }
}
