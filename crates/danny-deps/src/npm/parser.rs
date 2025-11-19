//! package.json parser

use crate::{
    Dependency, DependencyFile, DependencyManager, DependencyType, DependencyUpdate, Ecosystem,
    Error, Result, UpdateResult, VersionReq,
};
use danny_fs::FileSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PackageJson {
    name: String,
    version: String,

    #[serde(default)]
    dependencies: HashMap<String, String>,

    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,

    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,

    #[serde(default, rename = "optionalDependencies")]
    optional_dependencies: HashMap<String, String>,

    // Workspace fields
    #[serde(default)]
    workspaces: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum WorkspaceConfig {
    Simple(Vec<String>),
    Extended {
        packages: Vec<String>,
        #[serde(default)]
        nohoist: Vec<String>,
    },
}

/// npm/pnpm/yarn dependency manager
pub struct NpmDependencyManager;

impl NpmDependencyManager {
    /// Create a new npm dependency manager
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl DependencyManager for NpmDependencyManager {
    async fn parse<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<DependencyFile> {
        let content = fs.read_to_string(path).await?;
        let pkg: PackageJson = serde_json::from_str(&content).map_err(Error::Json)?;

        let mut dependencies = HashMap::new();

        // Runtime dependencies
        if !pkg.dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg
                .dependencies
                .iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Runtime,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Runtime, deps);
        }

        // Dev dependencies
        if !pkg.dev_dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg
                .dev_dependencies
                .iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Dev,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Dev, deps);
        }

        // Peer dependencies
        if !pkg.peer_dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg
                .peer_dependencies
                .iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Peer,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Peer, deps);
        }

        // Optional dependencies
        if !pkg.optional_dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg
                .optional_dependencies
                .iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Optional,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Optional, deps);
        }

        let is_workspace_root = pkg.workspaces.is_some();

        Ok(DependencyFile {
            path: path.to_path_buf(),
            ecosystem: Ecosystem::JavaScript,
            name: pkg.name,
            version: pkg.version,
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
        let mut pkg: serde_json::Value = serde_json::from_str(&content).map_err(Error::Json)?;
        let mut applied = vec![];

        for update in updates {
            let field = match update.dep_type {
                DependencyType::Runtime => "dependencies",
                DependencyType::Dev => "devDependencies",
                DependencyType::Peer => "peerDependencies",
                DependencyType::Optional => "optionalDependencies",
                _ => continue,
            };

            if let Some(deps) = pkg.get_mut(field).and_then(|v| v.as_object_mut()) {
                if let Some(old_version) = deps.get(&update.package) {
                    let old = old_version.as_str().unwrap_or("*").to_string();
                    deps.insert(
                        update.package.clone(),
                        serde_json::Value::String(update.new_version.clone()),
                    );

                    applied.push(crate::types::AppliedUpdate {
                        package: update.package.clone(),
                        old_version: old,
                        new_version: update.new_version.clone(),
                        dep_type: update.dep_type,
                    });
                }
            }
        }

        if !dry_run && !applied.is_empty() {
            // Pretty print with 2-space indentation (npm standard)
            let formatted = serde_json::to_string_pretty(&pkg).map_err(Error::Json)?;
            let updater = FileUpdater::new(false);
            updater.update_file(fs, path, formatted).await?;
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
        crate::npm::workspace::NpmWorkspace::get_members(fs, root).await
    }
}

impl Default for NpmDependencyManager {
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
    async fn test_parse_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        std::fs::write(
            &package_json,
            r#"
{
  "name": "test",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
"#,
        )
        .unwrap();

        let manager = NpmDependencyManager::new();
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());
        let manifest = manager.parse(&fs, &package_json).await.unwrap();

        assert_eq!(manifest.name, "test");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 2);

        let deps = manifest.dependencies_of_type(DependencyType::Runtime);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "react");
    }
}
