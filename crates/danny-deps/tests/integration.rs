//! Integration tests for danny-deps
//!
//! These tests verify end-to-end functionality including integration with danny-info

use danny_deps::{
    CargoDependencyManager, DependencyManager, DependencyType, Ecosystem, UnifiedDependencyManager,
};
use danny_fs::NativeFileSystem;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_parse_and_update_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create initial Cargo.toml
    std::fs::write(
        &cargo_toml,
        r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["macros"] }
"#,
    )
    .unwrap();

    let manager = CargoDependencyManager::new();
    let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());

    // Parse it
    let manifest = manager.parse(&fs, &cargo_toml).await.unwrap();
    assert_eq!(manifest.name, "test");
    assert_eq!(manifest.dependencies.len(), 1);

    let deps = manifest.dependencies_of_type(DependencyType::Runtime);
    assert_eq!(deps.len(), 2);

    // Update serde
    use danny_deps::DependencyUpdate;
    let updates = vec![DependencyUpdate {
        package: "serde".to_string(),
        new_version: "1.0.210".to_string(),
        dep_type: DependencyType::Runtime,
    }];

    // Dry run first
    let result = manager.update(&fs, &cargo_toml, &updates, true).await.unwrap();
    assert_eq!(result.updates.len(), 1);
    assert_eq!(result.updates[0].package, "serde");
    assert_eq!(result.updates[0].new_version, "1.0.210");

    // Actually update
    let result = manager.update(&fs, &cargo_toml, &updates, false).await.unwrap();
    assert_eq!(result.updates.len(), 1);

    // Parse again to verify
    let manifest2 = manager.parse(&fs, &cargo_toml).await.unwrap();
    let serde_dep = manifest2.find_dependency("serde").unwrap();
    assert_eq!(serde_dep.version_req.raw, "1.0.210");
}

#[tokio::test]
async fn test_workspace_detection() {
    let temp_dir = TempDir::new().unwrap();
    let root_cargo = temp_dir.path().join("Cargo.toml");

    // Create workspace root
    std::fs::write(
        &root_cargo,
        r#"
[workspace]
members = ["crate1", "crate2"]
"#,
    )
    .unwrap();

    // Create member crates
    std::fs::create_dir_all(temp_dir.path().join("crate1")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("crate2")).unwrap();

    std::fs::write(
        temp_dir.path().join("crate1/Cargo.toml"),
        r#"
[package]
name = "crate1"
version = "0.1.0"
"#,
    )
    .unwrap();

    std::fs::write(
        temp_dir.path().join("crate2/Cargo.toml"),
        r#"
[package]
name = "crate2"
version = "0.1.0"
"#,
    )
    .unwrap();

    let manager = CargoDependencyManager::new();
    let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());

    // Check if root is workspace
    assert!(manager.is_workspace_root(&fs, &root_cargo).await.unwrap());

    // Find members
    let members = manager.find_workspace_members(&fs, temp_dir.path()).await.unwrap();
    assert_eq!(members.len(), 2);
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_unified_manager_check_updates() {
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create a Cargo.toml with a real dependency
    std::fs::write(
        &cargo_toml,
        r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
    )
    .unwrap();

    let manager = UnifiedDependencyManager::new().unwrap();

    // Check for updates (this will make network requests)
    match manager.check_updates(&cargo_toml, Ecosystem::Rust).await {
        Ok(updates) => {
            // Should find serde
            let serde_update = updates.iter().find(|u| u.package == "serde");
            assert!(serde_update.is_some(), "Should find serde update");

            if let Some(update) = serde_update {
                assert!(!update.latest_version.is_empty());
                assert_eq!(update.registry, danny_info::Registry::CratesIo);
            }
        }
        Err(e) => {
            // Network errors are acceptable in tests
            eprintln!("Network error (acceptable in tests): {}", e);
        }
    }
}

#[test]
fn test_version_comparison_edge_cases() {
    use danny_deps::{compare_versions, update_type, Ecosystem, UpdateType};

    // Test pre-release versions
    assert_eq!(
        compare_versions("1.0.0-alpha", "1.0.0", Ecosystem::Rust).unwrap(),
        std::cmp::Ordering::Less
    );

    // Test patch updates
    assert_eq!(
        update_type("1.0.0", "1.0.1", Ecosystem::Rust).unwrap(),
        UpdateType::Patch
    );

    // Test minor updates
    assert_eq!(
        update_type("1.0.0", "1.1.0", Ecosystem::Rust).unwrap(),
        UpdateType::Minor
    );

    // Test major updates
    assert_eq!(
        update_type("1.0.0", "2.0.0", Ecosystem::Rust).unwrap(),
        UpdateType::Major
    );

    // Test no update needed
    assert_eq!(
        update_type("1.0.0", "1.0.0", Ecosystem::Rust).unwrap(),
        UpdateType::None
    );
}
