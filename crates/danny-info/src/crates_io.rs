//! crates.io registry client

use crate::client::HttpClient;
use crate::error::{Error, Result};
use crate::repository::parse_repository_url;
use crate::types::{PackageInfo, Registry};
use serde::Deserialize;

const CRATES_IO_API_URL: &str = "https://crates.io/api/v1";

/// crates.io API response structure
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    name: String,
    description: Option<String>,
    max_version: String,
    repository: Option<String>,
    homepage: Option<String>,
    #[serde(default)]
    license: Option<String>,
}

/// Fetch package information from crates.io registry
pub async fn fetch_crates_io_package(
    client: &HttpClient,
    crate_name: &str,
) -> Result<PackageInfo> {
    // Validate crate name
    if crate_name.is_empty() {
        return Err(Error::InvalidPackageName("Crate name cannot be empty".to_string()));
    }

    let url = format!("{}/crates/{}", CRATES_IO_API_URL, crate_name);

    // Fetch crate metadata
    let response: CratesIoResponse = client
        .get_json(&url)
        .await
        .map_err(|e| {
            if e.to_string().contains("404") {
                Error::PackageNotFound(crate_name.to_string(), "crates.io".to_string())
            } else {
                e
            }
        })?;

    // Extract repository information
    let repository = response
        .crate_info
        .repository
        .as_ref()
        .and_then(|url| parse_repository_url(url).ok());

    Ok(PackageInfo {
        registry: Registry::CratesIo,
        name: response.crate_info.name,
        version: response.crate_info.max_version,
        description: response.crate_info.description,
        repository,
        homepage: response.crate_info.homepage,
        license: response.crate_info.license,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_crates_io_package() {
        let client = HttpClient::new().unwrap();
        let info = fetch_crates_io_package(&client, "serde").await.unwrap();

        assert_eq!(info.registry, Registry::CratesIo);
        assert_eq!(info.name, "serde");
        assert!(!info.version.is_empty());
        assert!(info.description.is_some());
        assert!(info.repository.is_some());
    }

    #[tokio::test]
    async fn test_invalid_crate_name() {
        let client = HttpClient::new().unwrap();
        let result = fetch_crates_io_package(&client, "").await;
        assert!(matches!(result, Err(Error::InvalidPackageName(_))));
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_nonexistent_crate() {
        let client = HttpClient::new().unwrap();
        let result = fetch_crates_io_package(&client, "this-crate-definitely-does-not-exist-12345").await;
        assert!(matches!(result, Err(Error::PackageNotFound(_, _))));
    }
}
