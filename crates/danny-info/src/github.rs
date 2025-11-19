//! GitHub API client for releases and changelog

use crate::client::HttpClient;
use crate::error::{Error, Result};
use crate::types::{Release, RepositoryUrl};
use std::env;

const GITHUB_API_URL: &str = "https://api.github.com";

/// Common changelog file names to search for
const CHANGELOG_FILES: &[&str] = &[
    "CHANGELOG.md",
    "CHANGELOG",
    "CHANGES.md",
    "CHANGES",
    "HISTORY.md",
    "HISTORY",
    "NEWS.md",
    "NEWS",
    "RELEASES.md",
    "RELEASES",
];

/// Fetch releases from a GitHub repository
///
/// Returns up to 100 releases (GitHub API default per_page limit).
/// Set GITHUB_TOKEN environment variable for higher rate limits.
pub async fn fetch_releases(client: &HttpClient, repo: &RepositoryUrl) -> Result<Vec<Release>> {
    let url = format!(
        "{}/repos/{}/{}/releases",
        GITHUB_API_URL, repo.owner, repo.repo
    );

    // Add GitHub token if available for higher rate limits
    let releases: Vec<Release> = if let Ok(token) = env::var("GITHUB_TOKEN") {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse()
                .map_err(|_| Error::other("Invalid GitHub token format"))?,
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github+json"
                .parse()
                .map_err(|_| Error::other("Invalid Accept header"))?,
        );

        client.get_json_with_headers(&url, headers).await?
    } else {
        // Without token - lower rate limit
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github+json"
                .parse()
                .map_err(|_| Error::other("Invalid Accept header"))?,
        );

        client.get_json_with_headers(&url, headers).await?
    };

    if releases.is_empty() {
        return Err(Error::GitHubApi(format!(
            "No releases found for {}/{}",
            repo.owner, repo.repo
        )));
    }

    Ok(releases)
}

/// Fetch changelog content from a GitHub repository
///
/// Tries common changelog file names in order.
/// Returns the raw markdown content of the first found changelog.
pub async fn fetch_changelog(client: &HttpClient, repo: &RepositoryUrl) -> Result<String> {
    // Try each common changelog filename
    for filename in CHANGELOG_FILES {
        match try_fetch_file(client, repo, filename).await {
            Ok(content) => return Ok(content),
            Err(_) => continue, // Try next filename
        }
    }

    Err(Error::ChangelogNotFound(repo.owner.clone(), repo.repo.clone()))
}

/// Try to fetch a specific file from the repository's default branch
async fn try_fetch_file(
    client: &HttpClient,
    repo: &RepositoryUrl,
    filename: &str,
) -> Result<String> {
    // Use raw.githubusercontent.com for direct file access
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/HEAD/{}",
        repo.owner, repo.repo, filename
    );

    client.get_text(&url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_releases() {
        let client = HttpClient::new().unwrap();
        let repo = RepositoryUrl::new(
            "facebook",
            "react",
            "https://github.com/facebook/react",
        );

        let releases = fetch_releases(&client, &repo).await.unwrap();
        assert!(!releases.is_empty());
        assert!(releases[0].tag_name.starts_with('v'));
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_changelog() {
        let client = HttpClient::new().unwrap();
        let repo = RepositoryUrl::new(
            "facebook",
            "react",
            "https://github.com/facebook/react",
        );

        let changelog = fetch_changelog(&client, &repo).await.unwrap();
        assert!(!changelog.is_empty());
        assert!(changelog.contains("##") || changelog.contains("###"));
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_nonexistent_changelog() {
        let client = HttpClient::new().unwrap();
        // Use a repo that likely doesn't have a changelog
        let repo = RepositoryUrl::new(
            "octocat",
            "Hello-World",
            "https://github.com/octocat/Hello-World",
        );

        let result = fetch_changelog(&client, &repo).await;
        assert!(matches!(result, Err(Error::ChangelogNotFound(_, _))));
    }
}
