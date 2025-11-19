//! npm/pnpm/yarn workspace detection

use crate::{Error, Result};
use danny_fs::FileSystem;
use glob::glob;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// npm/pnpm/yarn workspace utilities
pub struct NpmWorkspace;

impl NpmWorkspace {
    /// Detect workspace type and find root
    ///
    /// Checks for:
    /// - package.json with workspaces field
    /// - pnpm-workspace.yaml
    pub fn find_root(start: &Path) -> Result<Option<PathBuf>> {
        let mut current = start.to_path_buf();

        loop {
            // Check for package.json with workspaces field
            let pkg_json = current.join("package.json");
            if pkg_json.exists() {
                let content = std::fs::read_to_string(&pkg_json)?;
                let pkg: Value = serde_json::from_str(&content).map_err(crate::Error::Json)?;

                if pkg.get("workspaces").is_some() {
                    return Ok(Some(current));
                }
            }

            // Check for pnpm-workspace.yaml
            if current.join("pnpm-workspace.yaml").exists() {
                return Ok(Some(current));
            }

            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Get workspace members (supports npm, pnpm, yarn)
    ///
    /// # Arguments
    /// * `fs` - FileSystem instance for file operations
    /// * `root` - Path to the workspace root
    pub async fn get_members<F: FileSystem>(fs: &Arc<F>, root: &Path) -> Result<Vec<PathBuf>> {
        // Try pnpm-workspace.yaml first
        let pnpm_workspace = root.join("pnpm-workspace.yaml");
        if fs.exists(&pnpm_workspace).await? {
            return Self::get_pnpm_members(fs, root).await;
        }

        // Try package.json workspaces
        let pkg_json = root.join("package.json");
        if fs.exists(&pkg_json).await? {
            return Self::get_npm_members(fs, root).await;
        }

        Ok(vec![])
    }

    async fn get_pnpm_members<F: FileSystem>(fs: &Arc<F>, root: &Path) -> Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        struct PnpmWorkspace {
            packages: Vec<String>,
        }

        let content = fs.read_to_string(&root.join("pnpm-workspace.yaml")).await?;
        let workspace: PnpmWorkspace =
            serde_yaml::from_str(&content).map_err(crate::Error::Yaml)?;

        let mut members = vec![];
        for pattern in workspace.packages {
            let full_pattern = root.join(&pattern).join("package.json");
            if let Some(pattern_str) = full_pattern.to_str() {
                for path in glob(pattern_str)
                    .map_err(|e| Error::WorkspaceError(format!("Invalid glob pattern: {}", e)))?
                    .flatten()
                {
                    members.push(path);
                }
            }
        }

        Ok(members)
    }

    async fn get_npm_members<F: FileSystem>(fs: &Arc<F>, root: &Path) -> Result<Vec<PathBuf>> {
        let content = fs.read_to_string(&root.join("package.json")).await?;
        let pkg: Value = serde_json::from_str(&content).map_err(crate::Error::Json)?;

        let patterns = match pkg.get("workspaces") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect::<Vec<_>>(),
            Some(Value::Object(obj)) => obj
                .get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            _ => vec![],
        };

        let mut members = vec![];
        for pattern in patterns {
            let full_pattern = root.join(&pattern).join("package.json");
            if let Some(pattern_str) = full_pattern.to_str() {
                for path in glob(pattern_str)
                    .map_err(|e| Error::WorkspaceError(format!("Invalid glob pattern: {}", e)))?
                    .flatten()
                {
                    members.push(path);
                }
            }
        }

        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_pnpm_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let pnpm_workspace = temp_dir.path().join("pnpm-workspace.yaml");
        std::fs::write(&pnpm_workspace, "packages:\n  - 'packages/*'").unwrap();

        let root = NpmWorkspace::find_root(temp_dir.path()).unwrap();
        assert_eq!(root, Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn test_find_npm_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        std::fs::write(
            &package_json,
            r#"{"name": "test", "workspaces": ["packages/*"]}"#,
        )
        .unwrap();

        let root = NpmWorkspace::find_root(temp_dir.path()).unwrap();
        assert_eq!(root, Some(temp_dir.path().to_path_buf()));
    }
}
