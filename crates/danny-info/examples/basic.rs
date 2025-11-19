//! Basic usage example for danny-info
//!
//! Run with: cargo run --package danny-info --example basic

use danny_info::InfoClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== danny-info Basic Example ===\n");

    let client = InfoClient::new()?;

    // 1. Fetch npm package
    println!("1. Fetching React from npm...");
    let react = client.fetch_npm("react").await?;
    println!("   Name: {}", react.name);
    println!("   Version: {}", react.version);
    println!("   Description: {}", react.description.as_deref().unwrap_or("N/A"));
    println!("   License: {}", react.license.as_deref().unwrap_or("N/A"));

    // 2. Get repository and fetch releases
    if let Some(repo) = &react.repository {
        println!("   Repository: {}", repo.url);
        println!("\n2. Fetching releases from {}...", repo.url);

        match client.fetch_releases(repo).await {
            Ok(releases) => {
                println!("   Found {} releases", releases.len());
                println!("   Latest 5 releases:");
                for (i, release) in releases.iter().take(5).enumerate() {
                    let name = release.name.as_deref().unwrap_or("Unnamed");
                    let prerelease = if release.prerelease { " (prerelease)" } else { "" };
                    println!("   {}. {} - {}{}", i + 1, release.tag_name, name, prerelease);
                }
            }
            Err(e) => println!("   Error fetching releases: {}", e),
        }

        // 3. Fetch changelog
        println!("\n3. Fetching changelog...");
        match client.fetch_changelog(repo).await {
            Ok(changelog) => {
                let lines: Vec<&str> = changelog.lines().collect();
                println!("   Changelog found: {} lines, {} bytes", lines.len(), changelog.len());
                println!("   First 10 lines:");
                for line in lines.iter().take(10) {
                    println!("   {}", line);
                }
            }
            Err(e) => println!("   No changelog found: {}", e),
        }
    }

    // 4. Fetch Rust crate
    println!("\n4. Fetching serde from crates.io...");
    let serde = client.fetch_crates_io("serde").await?;
    println!("   Name: {}", serde.name);
    println!("   Version: {}", serde.version);
    println!("   Description: {}", serde.description.as_deref().unwrap_or("N/A"));
    if let Some(repo) = &serde.repository {
        println!("   Repository: {}", repo.url);
    }

    // 5. Fetch JSR package
    println!("\n5. Fetching @std/path from JSR...");
    match client.fetch_jsr("@std/path").await {
        Ok(std_path) => {
            println!("   Name: {}", std_path.name);
            println!("   Version: {}", std_path.version);
            println!("   Description: {}", std_path.description.as_deref().unwrap_or("N/A"));
            if let Some(repo) = &std_path.repository {
                println!("   Repository: {}", repo.url);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // 6. Test scoped npm package
    println!("\n6. Fetching @types/node from npm...");
    let types_node = client.fetch_npm("@types/node").await?;
    println!("   Name: {}", types_node.name);
    println!("   Version: {}", types_node.version);

    // 7. Test repository URL parsing
    println!("\n7. Testing repository URL parsing...");
    let test_urls = vec![
        "https://github.com/facebook/react",
        "https://github.com/facebook/react.git",
        "git+https://github.com/facebook/react.git",
        "git@github.com:facebook/react.git",
    ];

    for url in test_urls {
        match InfoClient::parse_repository_url(url) {
            Ok(parsed) => println!("   ✓ {} -> {}/{}", url, parsed.owner, parsed.repo),
            Err(e) => println!("   ✗ {} -> Error: {}", url, e),
        }
    }

    println!("\n=== Done! ===");
    Ok(())
}
