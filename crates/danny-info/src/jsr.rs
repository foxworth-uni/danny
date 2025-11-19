//! JSR (JavaScript Registry) client

use crate::client::HttpClient;
use crate::error::{Error, Result};
use crate::types::{PackageInfo, Registry};
use serde::Deserialize;

const JSR_API_URL: &str = "https://jsr.io";

/// JSR package metadata response
#[derive(Debug, Deserialize)]
struct JsrPackageMetadata {
    scope: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    latest: Option<String>,
    #[serde(rename = "githubRepository")]
    github_repository: Option<GithubRepository>,
}

#[derive(Debug, Deserialize)]
struct GithubRepository {
    owner: String,
    name: String,
}

/// Fetch package information from JSR registry
///
/// Package name should be in the format "@scope/package" (e.g., "@std/path")
pub async fn fetch_jsr_package(client: &HttpClient, package_name: &str) -> Result<PackageInfo> {
    // Validate package name format
    if !package_name.starts_with('@') || !package_name.contains('/') {
        return Err(Error::InvalidPackageName(
            format!("JSR package name must be in format @scope/package, got: {}", package_name)
        ));
    }

    // Split into scope and name
    let parts: Vec<&str> = package_name.trim_start_matches('@').split('/').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidPackageName(
            format!("Invalid JSR package name format: {}", package_name)
        ));
    }

    let scope = parts[0];
    let name = parts[1];

    let url = format!("{}/@{}/{}/meta.json", JSR_API_URL, scope, name);

    // Fetch package metadata
    let response: JsrPackageMetadata = client
        .get_json(&url)
        .await
        .map_err(|e| {
            if e.to_string().contains("404") {
                Error::PackageNotFound(package_name.to_string(), "jsr".to_string())
            } else {
                e
            }
        })?;

    // Extract version (use latest)
    let version = response.latest.unwrap_or_else(|| "unknown".to_string());

    // Extract repository information from GitHub repository field
    let repository = response.github_repository.as_ref().map(|gh| {
        let url = format!("https://github.com/{}/{}", gh.owner, gh.name);
        crate::types::RepositoryUrl::new(
            gh.owner.clone(),
            gh.name.clone(),
            url,
        )
    });

    Ok(PackageInfo {
        registry: Registry::Jsr,
        name: format!("@{}/{}", response.scope, response.name),
        version,
        description: response.description,
        repository,
        homepage: None,
        license: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_jsr_package() {
        let client = HttpClient::new().unwrap();
        let info = fetch_jsr_package(&client, "@std/path").await.unwrap();

        assert_eq!(info.registry, Registry::Jsr);
        assert!(info.name.starts_with("@std/"));
        assert!(!info.version.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_package_name_format() {
        let client = HttpClient::new().unwrap();

        // Missing @ prefix
        let result = fetch_jsr_package(&client, "std/path").await;
        assert!(matches!(result, Err(Error::InvalidPackageName(_))));

        // Missing scope separator
        let result = fetch_jsr_package(&client, "@stdpath").await;
        assert!(matches!(result, Err(Error::InvalidPackageName(_))));
    }
}
