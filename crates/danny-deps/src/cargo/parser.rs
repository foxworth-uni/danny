//! Cargo.toml parser using toml_edit for comment/format preservation

use crate::{
    Dependency, DependencyFile, DependencyManager, DependencyType, DependencyUpdate, Ecosystem,
    Error, Result, UpdateResult, VersionReq,
};
use danny_fs::FileSystem;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use toml_edit::{DocumentMut, Item, Value};

/// Cargo dependency manager
pub struct CargoDependencyManager;

impl CargoDependencyManager {
    /// Create a new Cargo dependency manager
    pub fn new() -> Self {
        Self
    }

    fn parse_dependency(&self, name: &str, value: &Item) -> Result<Dependency> {
        match value {
            // Simple: serde = "1.0"
            Item::Value(Value::String(s)) => Ok(Dependency {
                name: name.to_string(),
                version_req: VersionReq {
                    raw: s.value().to_string(),
                    ecosystem: Ecosystem::Rust,
                },
                dep_type: DependencyType::Runtime,
                features: vec![],
                workspace: false,
                source: None,
            }),
            // Table: serde = { version = "1.0", features = ["derive"] }
            Item::Value(Value::InlineTable(table)) => {
                let version = table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "*".to_string());

                let features = table
                    .get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let workspace = table
                    .get("workspace")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let source = table
                    .get("git")
                    .or_else(|| table.get("path"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Ok(Dependency {
                    name: name.to_string(),
                    version_req: VersionReq {
                        raw: version,
                        ecosystem: Ecosystem::Rust,
                    },
                    dep_type: DependencyType::Runtime,
                    features,
                    workspace,
                    source,
                })
            }
            _ => Err(Error::InvalidFormat(
                Path::new("Cargo.toml").to_path_buf(),
                format!("Invalid dependency format for {}", name),
            )),
        }
    }
}

#[async_trait::async_trait]
impl DependencyManager for CargoDependencyManager {
    async fn parse<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<DependencyFile> {
        let content = fs.read_to_string(path).await?;
        let doc = content.parse::<DocumentMut>().map_err(Error::TomlEdit)?;

        // Parse [package] section (optional for workspace-only manifests)
        let (name, version) = if let Some(package) = doc.get("package").and_then(|p| p.as_table()) {
            let name = package
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| {
                    Error::InvalidFormat(path.to_path_buf(), "Missing package.name".to_string())
                })?
                .to_string();

            let version = package
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0")
                .to_string();

            (name, version)
        } else {
            // Workspace-only manifest - use workspace name or default
            ("workspace".to_string(), "0.0.0".to_string())
        };

        // Parse dependencies
        let mut dependencies = HashMap::new();

        if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
            let runtime_deps: Vec<Dependency> = deps
                .iter()
                .filter_map(|(k, v)| self.parse_dependency(k, v).ok())
                .collect();
            dependencies.insert(DependencyType::Runtime, runtime_deps);
        }

        if let Some(dev_deps) = doc.get("dev-dependencies").and_then(|d| d.as_table()) {
            let dev_deps: Vec<Dependency> = dev_deps
                .iter()
                .filter_map(|(k, v)| self.parse_dependency(k, v).ok())
                .map(|mut d| {
                    d.dep_type = DependencyType::Dev;
                    d
                })
                .collect();
            dependencies.insert(DependencyType::Dev, dev_deps);
        }

        if let Some(build_deps) = doc.get("build-dependencies").and_then(|d| d.as_table()) {
            let build_deps: Vec<Dependency> = build_deps
                .iter()
                .filter_map(|(k, v)| self.parse_dependency(k, v).ok())
                .map(|mut d| {
                    d.dep_type = DependencyType::Build;
                    d
                })
                .collect();
            dependencies.insert(DependencyType::Build, build_deps);
        }

        // Check if workspace root
        let is_workspace_root = doc.get("workspace").is_some();

        Ok(DependencyFile {
            path: path.to_path_buf(),
            ecosystem: Ecosystem::Rust,
            name,
            version,
            dependencies,
            is_workspace_root,
            workspace_members: vec![],
        })
    }

    async fn update<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
        updates: &[DependencyUpdate],
        dry_run: bool,
    ) -> Result<UpdateResult> {
        use crate::update::FileUpdater;

        let content = fs.read_to_string(path).await?;
        let mut doc = content.parse::<DocumentMut>().map_err(Error::TomlEdit)?;
        let mut applied = vec![];

        for update in updates {
            let section = match update.dep_type {
                DependencyType::Runtime => "dependencies",
                DependencyType::Dev => "dev-dependencies",
                DependencyType::Build => "build-dependencies",
                _ => continue,
            };

            if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_mut()) {
                if let Some(dep_item) = deps.get_mut(&update.package) {
                    let old_version = match dep_item {
                        Item::Value(Value::String(s)) => s.value().to_string(),
                        Item::Value(Value::InlineTable(table)) => table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_string(),
                        _ => continue,
                    };

                    // Update the version
                    match dep_item {
                        Item::Value(Value::String(s)) => {
                            *s = toml_edit::Formatted::new(update.new_version.clone());
                        }
                        Item::Value(Value::InlineTable(table)) => {
                            if let Some(version) = table.get_mut("version") {
                                *version = Value::String(toml_edit::Formatted::new(
                                    update.new_version.clone(),
                                ));
                            }
                        }
                        _ => {}
                    }

                    applied.push(crate::types::AppliedUpdate {
                        package: update.package.clone(),
                        old_version,
                        new_version: update.new_version.clone(),
                        dep_type: update.dep_type,
                    });
                }
            }
        }

        if !dry_run && !applied.is_empty() {
            let updater = FileUpdater::new(false);
            updater.update_file(fs, path, doc.to_string()).await?;
        }

        Ok(UpdateResult {
            file: path.to_path_buf(),
            updates: applied,
            dry_run,
        })
    }

    async fn validate<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<()> {
        // Just try to parse it
        self.parse(fs, path).await?;
        Ok(())
    }

    async fn is_workspace_root<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<bool> {
        let manifest = self.parse(fs, path).await?;
        Ok(manifest.is_workspace_root)
    }

    async fn find_workspace_members<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        root: &Path,
    ) -> Result<Vec<std::path::PathBuf>> {
        crate::cargo::workspace::CargoWorkspace::get_members(fs, root).await
    }
}

impl Default for CargoDependencyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use danny_fs::NativeFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_simple_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
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

        let manager = CargoDependencyManager::new();
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());
        let manifest = manager.parse(&fs, &cargo_toml).await.unwrap();

        assert_eq!(manifest.name, "test");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.dependencies.len(), 1);

        let deps = manifest.dependencies_of_type(DependencyType::Runtime);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "serde");
        assert_eq!(deps[0].version_req.raw, "1.0");
    }

    #[tokio::test]
    async fn test_parse_table_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#,
        )
        .unwrap();

        let manager = CargoDependencyManager::new();
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());
        let manifest = manager.parse(&fs, &cargo_toml).await.unwrap();

        let deps = manifest.dependencies_of_type(DependencyType::Runtime);
        assert_eq!(deps[0].features, vec!["derive"]);
    }

    #[tokio::test]
    async fn test_parse_workspace_root() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[workspace]
members = ["crate1", "crate2"]
"#,
        )
        .unwrap();

        let manager = CargoDependencyManager::new();
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());
        let manifest = manager.parse(&fs, &cargo_toml).await.unwrap();

        assert!(manifest.is_workspace_root);
    }
}
