//! Changelog parsing example for danny-info
//!
//! Run with: cargo run --package danny-info --example changelog_parser

use danny_info::{InfoClient, RepositoryUrl};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== danny-info Changelog Parsing Example ===\n");

    let client = InfoClient::new()?;

    // Example repository (React has a good changelog)
    let repo = RepositoryUrl::new(
        "facebook",
        "react",
        "https://github.com/facebook/react"
    );

    println!("Repository: {}\n", repo.url);

    // 1. Fetch GitHub Releases (already includes dates!)
    println!("1. GitHub Releases (with dates):");
    println!("   --------------------------------");
    match client.fetch_releases(&repo).await {
        Ok(releases) => {
            println!("   Found {} releases\n", releases.len());
            for (i, release) in releases.iter().take(5).enumerate() {
                let date = release.published_at.as_deref().unwrap_or("Unknown date");
                let name = release.name.as_deref().unwrap_or(&release.tag_name);
                println!("   {}. {} - {}", i + 1, name, date);
                
                // Show a snippet of the release notes
                if let Some(body) = &release.body {
                    let first_line = body.lines().next().unwrap_or("");
                    if !first_line.is_empty() {
                        println!("      {}", first_line);
                    }
                }
                println!();
            }
        }
        Err(e) => println!("   Error: {}\n", e),
    }

    // 2. Fetch and parse CHANGELOG.md
    println!("2. Parsed CHANGELOG.md (structured by version and date):");
    println!("   -----------------------------------------------------");
    match client.fetch_parsed_changelog(&repo).await {
        Ok(parsed) => {
            // Show preamble if it exists
            if let Some(preamble) = &parsed.other_content {
                println!("   Preamble:\n   {}\n", preamble.lines().next().unwrap_or(""));
            }

            println!("   Found {} changelog entries\n", parsed.entries.len());
            
            // Show the first 5 entries
            for (i, entry) in parsed.entries.iter().take(5).enumerate() {
                let date = entry.date.as_deref().unwrap_or("No date");
                println!("   {}. Version {} - {}", i + 1, entry.version, date);
                println!("      Heading: {}", entry.heading);
                
                // Show first line of content
                let first_line = entry.content.lines().next().unwrap_or("");
                if !first_line.is_empty() {
                    println!("      {}", first_line);
                }
                println!();
            }

            // Show statistics
            println!("   Statistics:");
            let entries_with_dates = parsed.entries.iter()
                .filter(|e| e.date.is_some())
                .count();
            println!("   - Total entries: {}", parsed.entries.len());
            println!("   - Entries with dates: {}", entries_with_dates);
            println!("   - Entries without dates: {}", parsed.entries.len() - entries_with_dates);
        }
        Err(e) => println!("   Error: {}\n", e),
    }

    // 3. Demonstrate filtering by date
    println!("\n3. Filtering entries (example):");
    println!("   -----------------------------");
    match client.fetch_parsed_changelog(&repo).await {
        Ok(parsed) => {
            // Find entries from 2024
            let entries_2024: Vec<_> = parsed.entries.iter()
                .filter(|e| e.date.as_ref().map_or(false, |d| d.starts_with("2024")))
                .collect();
            
            println!("   Entries from 2024: {}", entries_2024.len());
            for entry in entries_2024.iter().take(3) {
                println!("   - {} ({})", entry.version, entry.date.as_deref().unwrap_or(""));
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\n=== Done! ===");
    Ok(())
}

