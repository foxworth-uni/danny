//! Example: Workspace detection and member discovery
//!
//! Run with: cargo run --package danny-deps --example workspace_demo

use danny_deps::{CargoDependencyManager, DependencyManager};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== danny-deps: Workspace Detection ===\n");

    let manager = CargoDependencyManager::new();
    let cargo_toml = Path::new("Cargo.toml");

    if !cargo_toml.exists() {
        println!("Cargo.toml not found in current directory");
        return Ok(());
    }

    // Check if this is a workspace root
    match manager.is_workspace_root(cargo_toml) {
        Ok(true) => {
            println!("✓ This is a workspace root\n");

            // Find workspace members
            match manager.find_workspace_members(cargo_toml.parent().unwrap()) {
                Ok(members) => {
                    println!("Workspace members ({}):", members.len());
                    for member in members {
                        println!("  - {}", member.display());
                    }
                }
                Err(e) => println!("Error finding members: {}", e),
            }
        }
        Ok(false) => {
            println!("This is not a workspace root");
            println!("(It's a regular package or we're in a workspace member)\n");

            // Try to find the workspace root
            use danny_deps::cargo::workspace::CargoWorkspace;
            match CargoWorkspace::find_root(cargo_toml.parent().unwrap()) {
                Ok(Some(root)) => {
                    println!("Found workspace root at: {}", root.display());
                }
                Ok(None) => println!("No workspace root found"),
                Err(e) => println!("Error finding workspace root: {}", e),
            }
        }
        Err(e) => println!("Error checking workspace: {}", e),
    }

    Ok(())
}

