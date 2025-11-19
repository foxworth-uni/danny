//! Repository URL parsing and validation

use crate::error::{Error, Result};
use crate::types::RepositoryUrl;
use url::Url;

/// Parse a repository URL string into a RepositoryUrl
///
/// Supports various GitHub URL formats:
/// - https://github.com/owner/repo
/// - https://github.com/owner/repo.git
/// - git+https://github.com/owner/repo.git
/// - git://github.com/owner/repo.git
/// - ssh://git@github.com/owner/repo.git
/// - git@github.com:owner/repo.git
pub fn parse_repository_url(url_str: &str) -> Result<RepositoryUrl> {
    // Remove common prefixes
    let url_str = url_str
        .trim()
        .strip_prefix("git+")
        .unwrap_or(url_str)
        .strip_suffix(".git")
        .unwrap_or(url_str);

    // Handle SSH format: git@github.com:owner/repo
    if let Some(ssh_part) = url_str.strip_prefix("git@") {
        return parse_ssh_url(ssh_part);
    }

    // Parse as normal URL
    let url = Url::parse(url_str)
        .map_err(|_| Error::InvalidRepositoryUrl(format!("Could not parse URL: {}", url_str)))?;

    // Check if it's GitHub
    let host = url
        .host_str()
        .ok_or_else(|| Error::InvalidRepositoryUrl(format!("No host found in URL: {}", url_str)))?;

    if !host.ends_with("github.com") {
        return Err(Error::UnsupportedRepositoryHost(host.to_string()));
    }

    // Extract owner and repo from path
    let path = url.path().trim_start_matches('/').trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() < 2 {
        return Err(Error::InvalidRepositoryUrl(format!(
            "Could not extract owner/repo from path: {}",
            path
        )));
    }

    let owner = parts[0].to_string();
    let repo = parts[1].to_string();

    Ok(RepositoryUrl::new(
        owner,
        repo,
        format!("https://github.com/{}/{}", parts[0], parts[1]),
    ))
}

/// Parse SSH-style URL: github.com:owner/repo
fn parse_ssh_url(ssh_part: &str) -> Result<RepositoryUrl> {
    let parts: Vec<&str> = ssh_part.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidRepositoryUrl(format!(
            "Invalid SSH URL format: git@{}",
            ssh_part
        )));
    }

    let host = parts[0];
    if !host.ends_with("github.com") {
        return Err(Error::UnsupportedRepositoryHost(host.to_string()));
    }

    let path = parts[1].trim_end_matches(".git");
    let path_parts: Vec<&str> = path.split('/').collect();

    if path_parts.len() != 2 {
        return Err(Error::InvalidRepositoryUrl(format!(
            "Could not extract owner/repo from SSH path: {}",
            path
        )));
    }

    let owner = path_parts[0].to_string();
    let repo = path_parts[1].to_string();

    Ok(RepositoryUrl::new(
        owner.clone(),
        repo.clone(),
        format!("https://github.com/{}/{}", owner, repo),
    ))
}

/// Extract repository URL from package.json-style repository field
///
/// The field can be either:
/// - A string: "https://github.com/owner/repo"
/// - An object: { "type": "git", "url": "https://github.com/owner/repo" }
pub fn extract_repo_from_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(obj) => obj.get("url").and_then(|v| v.as_str()).map(String::from),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_url() {
        let result = parse_repository_url("https://github.com/facebook/react").unwrap();
        assert_eq!(result.owner, "facebook");
        assert_eq!(result.repo, "react");
        assert_eq!(result.url, "https://github.com/facebook/react");
    }

    #[test]
    fn test_parse_https_url_with_git_suffix() {
        let result = parse_repository_url("https://github.com/facebook/react.git").unwrap();
        assert_eq!(result.owner, "facebook");
        assert_eq!(result.repo, "react");
    }

    #[test]
    fn test_parse_git_plus_https() {
        let result = parse_repository_url("git+https://github.com/facebook/react.git").unwrap();
        assert_eq!(result.owner, "facebook");
        assert_eq!(result.repo, "react");
    }

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_repository_url("git@github.com:facebook/react.git").unwrap();
        assert_eq!(result.owner, "facebook");
        assert_eq!(result.repo, "react");
    }

    #[test]
    fn test_parse_unsupported_host() {
        let result = parse_repository_url("https://gitlab.com/owner/repo");
        assert!(matches!(result, Err(Error::UnsupportedRepositoryHost(_))));
    }

    #[test]
    fn test_extract_repo_from_string() {
        let json = serde_json::json!("https://github.com/owner/repo");
        let result = extract_repo_from_json(&json);
        assert_eq!(result, Some("https://github.com/owner/repo".to_string()));
    }

    #[test]
    fn test_extract_repo_from_object() {
        let json = serde_json::json!({
            "type": "git",
            "url": "https://github.com/owner/repo"
        });
        let result = extract_repo_from_json(&json);
        assert_eq!(result, Some("https://github.com/owner/repo".to_string()));
    }
}
