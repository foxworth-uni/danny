//! npm registry client

use crate::client::HttpClient;
use crate::error::{Error, Result};
use crate::repository::{extract_repo_from_json, parse_repository_url};
use crate::types::{PackageInfo, Registry};
use serde::Deserialize;

const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org";

/// npm registry API response structure
#[derive(Debug, Deserialize)]
struct NpmPackageResponse {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
    repository: Option<serde_json::Value>,
    homepage: Option<String>,
    license: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DistTags {
    latest: String,
}

/// Fetch package information from npm registry
pub async fn fetch_npm_package(client: &HttpClient, package_name: &str) -> Result<PackageInfo> {
    // Validate package name (basic validation)
    if package_name.is_empty() {
        return Err(Error::InvalidPackageName("Package name cannot be empty".to_string()));
    }

    // Encode package name for URL (handle scoped packages like @scope/name)
    let encoded_name = if package_name.starts_with('@') {
        package_name.replace('/', "%2F")
    } else {
        package_name.to_string()
    };

    let url = format!("{}/{}", NPM_REGISTRY_URL, encoded_name);

    // Fetch package metadata
    let response: NpmPackageResponse = client
        .get_json(&url)
        .await
        .map_err(|e| {
            if e.to_string().contains("404") {
                Error::PackageNotFound(package_name.to_string(), "npm".to_string())
            } else {
                e
            }
        })?;

    // Extract repository information
    let repository = response
        .repository
        .as_ref()
        .and_then(extract_repo_from_json)
        .and_then(|url| parse_repository_url(&url).ok());

    // Extract license
    let license = response.license.as_ref().and_then(|l| match l {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(obj) => {
            obj.get("type").and_then(|v| v.as_str()).map(String::from)
        }
        _ => None,
    });

    Ok(PackageInfo {
        registry: Registry::Npm,
        name: response.name,
        version: response.dist_tags.latest,
        description: response.description,
        repository,
        homepage: response.homepage,
        license,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_npm_package() {
        let client = HttpClient::new().unwrap();
        let info = fetch_npm_package(&client, "react").await.unwrap();

        assert_eq!(info.registry, Registry::Npm);
        assert_eq!(info.name, "react");
        assert!(!info.version.is_empty());
        assert!(info.description.is_some());
        assert!(info.repository.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_scoped_npm_package() {
        let client = HttpClient::new().unwrap();
        let info = fetch_npm_package(&client, "@types/node").await.unwrap();

        assert_eq!(info.registry, Registry::Npm);
        assert_eq!(info.name, "@types/node");
        assert!(!info.version.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_package_name() {
        let client = HttpClient::new().unwrap();
        let result = fetch_npm_package(&client, "").await;
        assert!(matches!(result, Err(Error::InvalidPackageName(_))));
    }
}
