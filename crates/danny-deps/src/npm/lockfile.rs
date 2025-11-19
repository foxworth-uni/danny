//! npm/pnpm/yarn lockfile parsers

use crate::{Error, LockfileParser, LockedDependencies, LockedPackage, Result};
use danny_fs::FileSystem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// package-lock.json parser (npm v7+)
pub struct NpmLockfileParser;

impl NpmLockfileParser {
    /// Create a new npm lockfile parser
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct PackageLock {
    #[serde(rename = "lockfileVersion")]
    _lockfile_version: u8,
    packages: HashMap<String, PackageLockEntry>,
}

#[derive(Debug, Deserialize)]
struct PackageLockEntry {
    version: String,
    resolved: Option<String>,
    integrity: Option<String>,
}

#[async_trait::async_trait]
impl LockfileParser for NpmLockfileParser {
    async fn parse_lockfile<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
    ) -> Result<LockedDependencies> {
        let content = fs.read_to_string(path).await?;
        let lock: PackageLock = serde_json::from_str(&content)
            .map_err(|e| Error::Json(e))?;

        let packages = lock
            .packages
            .iter()
            .filter_map(|(key, entry)| {
                // Skip root entry (empty key)
                if key.is_empty() {
                    return None;
                }

                // Extract package name from "node_modules/foo" or "node_modules/@scope/foo"
                let name = if let Some(stripped) = key.strip_prefix("node_modules/") {
                    stripped.to_string()
                } else {
                    return None;
                };

                Some((
                    name.clone(),
                    LockedPackage {
                        name,
                        version: entry.version.clone(),
                        checksum: entry.integrity.clone(),
                        resolved: entry.resolved.clone(),
                    },
                ))
            })
            .collect();

        Ok(LockedDependencies { packages })
    }

    async fn verify_integrity<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<()> {
        // For now, just parse the file
        // Full integrity verification would require fetching packages
        self.parse_lockfile(fs, path).await?;
        Ok(())
    }
}

impl Default for NpmLockfileParser {
    fn default() -> Self {
        Self::new()
    }
}

/// pnpm-lock.yaml parser
pub struct PnpmLockfileParser;

impl PnpmLockfileParser {
    /// Create a new pnpm lockfile parser
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct PnpmLock {
    #[serde(rename = "lockfileVersion")]
    _lockfile_version: String,
    packages: HashMap<String, PnpmLockEntry>,
}

#[derive(Debug, Deserialize)]
struct PnpmLockEntry {
    resolution: PnpmResolution,
}

#[derive(Debug, Deserialize)]
struct PnpmResolution {
    integrity: String,
    tarball: Option<String>,
}

#[async_trait::async_trait]
impl LockfileParser for PnpmLockfileParser {
    async fn parse_lockfile<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
    ) -> Result<LockedDependencies> {
        let content = fs.read_to_string(path).await?;
        let lock: PnpmLock = serde_yaml::from_str(&content)
            .map_err(|e| Error::Yaml(e))?;

        // pnpm uses format: "/foo/1.0.0" as key
        let packages = lock
            .packages
            .iter()
            .filter_map(|(key, entry)| {
                let parts: Vec<&str> = key.trim_start_matches('/').split('/').collect();
                if parts.len() < 2 {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts[1].to_string();

                Some((
                    name.clone(),
                    LockedPackage {
                        name,
                        version,
                        checksum: Some(entry.resolution.integrity.clone()),
                        resolved: entry.resolution.tarball.clone(),
                    },
                ))
            })
            .collect();

        Ok(LockedDependencies { packages })
    }

    async fn verify_integrity<F: FileSystem>(&self, fs: &Arc<F>, path: &Path) -> Result<()> {
        self.parse_lockfile(fs, path).await?;
        Ok(())
    }
}

impl Default for PnpmLockfileParser {
    fn default() -> Self {
        Self::new()
    }
}

/// yarn.lock parser (basic support)
///
/// Note: yarn.lock uses a custom format that's harder to parse.
/// This is a simplified parser that extracts basic information.
pub struct YarnLockfileParser;

impl YarnLockfileParser {
    /// Create a new yarn lockfile parser
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LockfileParser for YarnLockfileParser {
    async fn parse_lockfile<F: FileSystem>(
        &self,
        _fs: &Arc<F>,
        _path: &Path,
    ) -> Result<LockedDependencies> {
        // Yarn lockfile format is complex and uses a custom parser
        // For now, return empty dependencies
        // TODO: Implement proper yarn.lock parsing
        Ok(LockedDependencies {
            packages: HashMap::new(),
        })
    }

    async fn verify_integrity<F: FileSystem>(&self, _fs: &Arc<F>, _path: &Path) -> Result<()> {
        // TODO: Implement yarn.lock integrity verification
        Ok(())
    }
}

impl Default for YarnLockfileParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_package_lock() {
        let temp_dir = TempDir::new().unwrap();
        let lockfile = temp_dir.path().join("package-lock.json");
        std::fs::write(
            &lockfile,
            r#"
{
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "test",
      "version": "1.0.0"
    },
    "node_modules/react": {
      "version": "18.0.0",
      "resolved": "https://registry.npmjs.org/react/-/react-18.0.0.tgz",
      "integrity": "sha512-..."
    }
  }
}
"#,
        )
        .unwrap();

        let parser = NpmLockfileParser::new();
        let result = parser.parse_lockfile(&lockfile).unwrap();

        assert!(result.packages.contains_key("react"));
        assert_eq!(result.packages["react"].version, "18.0.0");
    }
}

