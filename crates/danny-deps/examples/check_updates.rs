//! Example: Check for dependency updates using unified API
//!
//! Run with: cargo run --package danny-deps --example check_updates

use danny_deps::{Ecosystem, UnifiedDependencyManager};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== danny-deps: Check for Updates ===\n");

    let manager = UnifiedDependencyManager::new()?;

    // Check Rust dependencies
    let cargo_toml = Path::new("Cargo.toml");
    if cargo_toml.exists() {
        println!("Checking Rust dependencies in Cargo.toml...\n");
        match manager.check_updates(cargo_toml, Ecosystem::Rust).await {
            Ok(updates) => {
                println!("Found {} potential updates\n", updates.len());

                // Show breaking updates
                let breaking = UnifiedDependencyManager::breaking_updates(&updates);
                if !breaking.is_empty() {
                    println!("⚠️  Breaking Updates (Major):");
                    for update in breaking {
                        println!(
                            "  {}: {} -> {}",
                            update.package, update.current_req.raw, update.latest_version
                        );
                        if !update.changelog_entries.is_empty() {
                            println!("    Changelog: {} entries", update.changelog_entries.len());
                        }
                    }
                    println!();
                }

                // Show safe updates
                let safe = UnifiedDependencyManager::safe_updates(&updates);
                if !safe.is_empty() {
                    println!("✅ Safe Updates (Minor/Patch):");
                    for update in safe {
                        println!(
                            "  {}: {} -> {}",
                            update.package, update.current_req.raw, update.latest_version
                        );
                    }
                    println!();
                }

                // Show updates that don't satisfy requirement
                let unsatisfied: Vec<_> = updates
                    .iter()
                    .filter(|u| {
                        !u.satisfies_requirement && u.update_type != danny_deps::UpdateType::None
                    })
                    .collect();
                if !unsatisfied.is_empty() {
                    println!("📦 Updates Requiring Requirement Change:");
                    for update in unsatisfied {
                        println!(
                            "  {}: {} -> {} (current req: {})",
                            update.package,
                            update.current_req.raw,
                            update.latest_version,
                            update.current_req.raw
                        );
                    }
                }
            }
            Err(e) => println!("Error checking updates: {}\n", e),
        }
    }

    // Check JavaScript dependencies
    let package_json = Path::new("package.json");
    if package_json.exists() {
        println!("Checking JavaScript dependencies in package.json...\n");
        match manager
            .check_updates(package_json, Ecosystem::JavaScript)
            .await
        {
            Ok(updates) => {
                println!("Found {} potential updates\n", updates.len());

                // Show breaking updates
                let breaking = UnifiedDependencyManager::breaking_updates(&updates);
                if !breaking.is_empty() {
                    println!("⚠️  Breaking Updates (Major):");
                    for update in breaking {
                        println!(
                            "  {}: {} -> {}",
                            update.package, update.current_req.raw, update.latest_version
                        );
                    }
                    println!();
                }

                // Show safe updates
                let safe = UnifiedDependencyManager::safe_updates(&updates);
                if !safe.is_empty() {
                    println!("✅ Safe Updates (Minor/Patch):");
                    for update in safe.iter().take(10) {
                        println!(
                            "  {}: {} -> {}",
                            update.package, update.current_req.raw, update.latest_version
                        );
                    }
                    if safe.len() > 10 {
                        println!("  ... and {} more", safe.len() - 10);
                    }
                }
            }
            Err(e) => println!("Error checking updates: {}\n", e),
        }
    }

    Ok(())
}
