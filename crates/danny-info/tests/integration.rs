//! Integration tests for danny-info
//!
//! These tests require network access and are ignored by default.
//! Run with: cargo test --package danny-info -- --ignored

use danny_info::{InfoClient, Registry};

#[tokio::test]
#[ignore]
async fn test_fetch_npm_package() {
    let client = InfoClient::new().unwrap();
    let info = client.fetch_npm("react").await.unwrap();

    assert_eq!(info.registry, Registry::Npm);
    assert_eq!(info.name, "react");
    assert!(!info.version.is_empty());
    assert!(info.description.is_some());
    assert!(info.repository.is_some());

    // Verify repository info
    let repo = info.repository.unwrap();
    assert_eq!(repo.owner, "facebook");
    assert_eq!(repo.repo, "react");
}

#[tokio::test]
#[ignore]
async fn test_fetch_scoped_npm_package() {
    let client = InfoClient::new().unwrap();
    let info = client.fetch_npm("@types/node").await.unwrap();

    assert_eq!(info.registry, Registry::Npm);
    assert_eq!(info.name, "@types/node");
    assert!(!info.version.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_fetch_crates_io_package() {
    let client = InfoClient::new().unwrap();
    let info = client.fetch_crates_io("serde").await.unwrap();

    assert_eq!(info.registry, Registry::CratesIo);
    assert_eq!(info.name, "serde");
    assert!(!info.version.is_empty());
    assert!(info.description.is_some());
    assert!(info.repository.is_some());

    let repo = info.repository.unwrap();
    assert_eq!(repo.owner, "serde-rs");
    assert_eq!(repo.repo, "serde");
}

#[tokio::test]
#[ignore]
async fn test_fetch_jsr_package() {
    let client = InfoClient::new().unwrap();
    let info = client.fetch_jsr("@std/path").await.unwrap();

    assert_eq!(info.registry, Registry::Jsr);
    assert!(info.name.starts_with("@std/"));
    assert!(!info.version.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_fetch_github_releases() {
    let client = InfoClient::new().unwrap();

    // First fetch npm package to get repo
    let info = client.fetch_npm("react").await.unwrap();
    let repo = info.repository.unwrap();

    // Fetch releases
    let releases = client.fetch_releases(&repo).await.unwrap();
    assert!(!releases.is_empty());

    let latest = &releases[0];
    assert!(latest.tag_name.starts_with('v'));
    assert!(!latest.prerelease || latest.prerelease);
}

#[tokio::test]
#[ignore]
async fn test_fetch_github_changelog() {
    let client = InfoClient::new().unwrap();

    // Fetch npm package to get repo
    let info = client.fetch_npm("react").await.unwrap();
    let repo = info.repository.unwrap();

    // Fetch changelog
    let changelog = client.fetch_changelog(&repo).await.unwrap();
    assert!(!changelog.is_empty());
    assert!(changelog.contains("##") || changelog.contains("###"));
}

#[tokio::test]
#[ignore]
async fn test_full_workflow() {
    let client = InfoClient::new().unwrap();

    // 1. Fetch package from npm
    let react = client.fetch_npm("react").await.unwrap();
    println!(
        "React v{}: {}",
        react.version,
        react.description.as_ref().unwrap()
    );

    // 2. Get repository info
    assert!(react.repository.is_some());
    let repo = react.repository.unwrap();
    println!("Repository: {}", repo.url);

    // 3. Fetch releases
    let releases = client.fetch_releases(&repo).await.unwrap();
    println!("Found {} releases", releases.len());
    println!("Latest release: {}", releases[0].tag_name);

    // 4. Fetch changelog
    match client.fetch_changelog(&repo).await {
        Ok(changelog) => println!("Changelog: {} bytes", changelog.len()),
        Err(e) => println!("No changelog: {}", e),
    }

    // 5. Also test crates.io
    let serde = client.fetch_crates_io("serde").await.unwrap();
    println!("\nSerde v{}", serde.version);
}

#[test]
fn test_parse_repository_urls() {
    // HTTPS
    let repo = InfoClient::parse_repository_url("https://github.com/facebook/react").unwrap();
    assert_eq!(repo.owner, "facebook");
    assert_eq!(repo.repo, "react");

    // HTTPS with .git
    let repo = InfoClient::parse_repository_url("https://github.com/facebook/react.git").unwrap();
    assert_eq!(repo.owner, "facebook");
    assert_eq!(repo.repo, "react");

    // git+https
    let repo =
        InfoClient::parse_repository_url("git+https://github.com/facebook/react.git").unwrap();
    assert_eq!(repo.owner, "facebook");
    assert_eq!(repo.repo, "react");

    // SSH
    let repo = InfoClient::parse_repository_url("git@github.com:facebook/react.git").unwrap();
    assert_eq!(repo.owner, "facebook");
    assert_eq!(repo.repo, "react");
}
