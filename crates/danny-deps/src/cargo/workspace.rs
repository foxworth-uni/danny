//! Cargo workspace detection

use crate::{Error, Result};
use danny_fs::FileSystem;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml_edit::DocumentMut;

/// Cargo workspace utilities
pub struct CargoWorkspace;

impl CargoWorkspace {
    /// Find workspace root starting from a given path
    ///
    /// Walks up the directory tree looking for a Cargo.toml with a [workspace] section.
    pub fn find_root(start: &Path) -> Result<Option<PathBuf>> {
        let mut current = start.to_path_buf();

        loop {
            let manifest_path = current.join("Cargo.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)?;
                let doc = content
                    .parse::<DocumentMut>()
                    .map_err(|e| crate::Error::TomlEdit(e))?;

                if doc.get("workspace").is_some() {
                    return Ok(Some(current));
                }
            }

            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Get workspace members from a workspace root
    ///
    /// # Arguments
    /// * `fs` - FileSystem instance for file operations
    /// * `root` - Path to the workspace root (must contain Cargo.toml with [workspace])
    ///
    /// # Errors
    /// Returns an error if the root is not a valid workspace or members cannot be parsed
    pub async fn get_members<F: FileSystem>(fs: &Arc<F>, root: &Path) -> Result<Vec<PathBuf>> {
        let manifest_path = root.join("Cargo.toml");
        let content = fs.read_to_string(&manifest_path).await?;
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| crate::Error::TomlEdit(e))?;

        let workspace = doc
            .get("workspace")
            .and_then(|w| w.as_table())
            .ok_or_else(|| Error::WorkspaceError("Not a workspace root".to_string()))?;

        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .ok_or_else(|| Error::WorkspaceError("No members field".to_string()))?;

        let member_paths: Vec<PathBuf> = members
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| root.join(s).join("Cargo.toml"))
            .collect();

        Ok(member_paths)
    }

    /// Check if a path is a workspace root
    pub async fn is_workspace_root<F: FileSystem>(fs: &Arc<F>, path: &Path) -> Result<bool> {
        let manifest_path = path.join("Cargo.toml");
        if !fs.exists(&manifest_path).await? {
            return Ok(false);
        }

        let content = fs.read_to_string(&manifest_path).await?;
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| crate::Error::TomlEdit(e))?;

        Ok(doc.get("workspace").is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_workspace_root() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        // Not a workspace
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"
"#,
        )
        .unwrap();
        assert!(!CargoWorkspace::is_workspace_root(temp_dir.path()).unwrap());

        // Is a workspace
        std::fs::write(
            &cargo_toml,
            r#"
[workspace]
members = ["crate1"]
"#,
        )
        .unwrap();
        assert!(CargoWorkspace::is_workspace_root(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_get_members() {
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

        let members = CargoWorkspace::get_members(temp_dir.path()).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members[0].ends_with("crate1/Cargo.toml"));
        assert!(members[1].ends_with("crate2/Cargo.toml"));
    }
}

