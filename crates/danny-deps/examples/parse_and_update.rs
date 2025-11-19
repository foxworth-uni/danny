//! Example: Parse dependencies and update them
//!
//! Run with: cargo run --package danny-deps --example parse_and_update

use danny_deps::{CargoDependencyManager, DependencyManager, DependencyType, DependencyUpdate};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== danny-deps: Parse and Update Dependencies ===\n");

    let manager = CargoDependencyManager::new();
    let cargo_toml = Path::new("Cargo.toml");

    if !cargo_toml.exists() {
        println!("Cargo.toml not found in current directory");
        return Ok(());
    }

    // Parse the manifest
    println!("Parsing Cargo.toml...");
    let manifest = manager.parse(cargo_toml)?;
    println!("Package: {} v{}\n", manifest.name, manifest.version);

    // List all dependencies
    println!("Dependencies:");
    for dep in manifest.all_dependencies() {
        let dep_type_str = match dep.dep_type {
            DependencyType::Runtime => "runtime",
            DependencyType::Dev => "dev",
            DependencyType::Build => "build",
            _ => "other",
        };
        println!(
            "  {} ({}) = {}",
            dep.name, dep_type_str, dep.version_req.raw
        );
    }
    println!();

    // Example: Update serde to a specific version (dry-run)
    println!("Example: Updating serde to 1.0.210 (dry-run)...");
    let updates = vec![DependencyUpdate {
        package: "serde".to_string(),
        new_version: "1.0.210".to_string(),
        dep_type: DependencyType::Runtime,
    }];

    match manager.update(cargo_toml, &updates, true) {
        Ok(result) => {
            if result.updates.is_empty() {
                println!("  serde not found in dependencies");
            } else {
                println!("  Would update:");
                for update in &result.updates {
                    println!(
                        "    {}: {} -> {}",
                        update.package, update.old_version, update.new_version
                    );
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    Ok(())
}

