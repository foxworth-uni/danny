//! Integration with danny-info for unified dependency management

use crate::{DependencyManager, Ecosystem, Error, Result, UpdateType, VersionReq};
use danny_fs::NativeFileSystem;
use danny_info::{InfoClient, Registry};
use std::path::Path;
use std::sync::Arc;

/// Update recommendation combining local and remote data
#[derive(Debug, Clone)]
pub struct UpdateRecommendation {
    /// Package name
    pub package: String,
    /// Current version requirement
    pub current_req: VersionReq,
    /// Current installed version (from lockfile, if available)
    pub current_installed: Option<String>,
    /// Latest available version
    pub latest_version: String,
    /// Update type (major, minor, patch)
    pub update_type: UpdateType,
    /// Whether update satisfies current requirement
    pub satisfies_requirement: bool,
    /// Registry information
    pub registry: Registry,
    /// Changelog entries (if available)
    pub changelog_entries: Vec<danny_info::ChangelogEntry>,
    /// GitHub releases (if available)
    pub releases: Vec<danny_info::Release>,
}

/// Unified dependency manager combining local parsing and remote fetching
pub struct UnifiedDependencyManager {
    info_client: InfoClient,
}

impl UnifiedDependencyManager {
    /// Create a new unified dependency manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            info_client: InfoClient::new()
                .map_err(|e| Error::Other(format!("Failed to create InfoClient: {}", e)))?,
        })
    }

    /// Check for updates in a dependency file
    ///
    /// This combines local dependency parsing with remote package data fetching
    /// to provide comprehensive update recommendations.
    ///
    /// # Arguments
    /// * `manifest_path` - Path to dependency file (Cargo.toml or package.json)
    /// * `ecosystem` - The ecosystem to use for parsing
    ///
    /// # Example
    /// ```rust,no_run
    /// use danny_deps::{UnifiedDependencyManager, Ecosystem};
    /// use std::path::Path;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let manager = UnifiedDependencyManager::new()?;
    /// let updates = manager.check_updates(Path::new("Cargo.toml"), Ecosystem::Rust).await?;
    ///
    /// for update in updates {
    ///     println!("{}: {} -> {}", update.package, update.current_req.raw, update.latest_version);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_updates(
        &self,
        manifest_path: &Path,
        ecosystem: Ecosystem,
    ) -> Result<Vec<UpdateRecommendation>> {
        // Create FileSystem scoped to manifest directory
        let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        let fs = Arc::new(
            NativeFileSystem::new(manifest_dir)
                .map_err(|e| Error::Other(format!("Failed to create filesystem: {}", e)))?,
        );

        // Parse local dependency file
        let manifest = match ecosystem {
            Ecosystem::Rust => {
                use crate::CargoDependencyManager;
                CargoDependencyManager::new()
                    .parse(&fs, manifest_path)
                    .await?
            }
            Ecosystem::JavaScript => {
                use crate::NpmDependencyManager;
                NpmDependencyManager::new()
                    .parse(&fs, manifest_path)
                    .await?
            }
        };

        let mut recommendations = Vec::new();

        // Check each dependency
        for dep in manifest.all_dependencies() {
            // Skip workspace dependencies
            if dep.workspace {
                continue;
            }

            // Fetch latest version from registry
            let latest_info = match ecosystem {
                Ecosystem::Rust => self
                    .info_client
                    .fetch_crates_io(&dep.name)
                    .await
                    .map_err(|e| Error::Other(format!("Failed to fetch {}: {}", dep.name, e)))?,
                Ecosystem::JavaScript => self
                    .info_client
                    .fetch_npm(&dep.name)
                    .await
                    .map_err(|e| Error::Other(format!("Failed to fetch {}: {}", dep.name, e)))?,
            };

            // Parse versions
            let latest_version = latest_info.version.clone();

            // Determine update type
            let update_type = if let Some(current) = &dep
                .version_req
                .raw
                .strip_prefix("^")
                .or_else(|| dep.version_req.raw.strip_prefix("~"))
                .or_else(|| dep.version_req.raw.strip_prefix(">="))
                .or(Some(dep.version_req.raw.as_str()))
            {
                crate::version::update_type(current, &latest_version, ecosystem)
                    .unwrap_or(UpdateType::None)
            } else {
                UpdateType::None
            };

            // Check if latest satisfies requirement
            let parsed_req =
                crate::version::ParsedVersionReq::parse(&dep.version_req.raw, ecosystem).ok();
            let satisfies_requirement = parsed_req
                .as_ref()
                .and_then(|req| req.matches(&latest_version).ok())
                .unwrap_or(false);

            // Fetch changelog if repository is available
            let changelog_entries = if let Some(repo) = &latest_info.repository {
                self.info_client
                    .fetch_parsed_changelog(repo)
                    .await
                    .map(|parsed| parsed.entries)
                    .unwrap_or_default()
            } else {
                vec![]
            };

            // Fetch releases if repository is available
            let releases = if let Some(repo) = &latest_info.repository {
                self.info_client
                    .fetch_releases(repo)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            };

            recommendations.push(UpdateRecommendation {
                package: dep.name.clone(),
                current_req: dep.version_req.clone(),
                current_installed: None, // TODO: Parse from lockfile
                latest_version,
                update_type,
                satisfies_requirement,
                registry: latest_info.registry,
                changelog_entries,
                releases,
            });
        }

        Ok(recommendations)
    }

    /// Get update recommendations filtered by update type
    pub fn filter_by_type(
        recommendations: &[UpdateRecommendation],
        update_type: UpdateType,
    ) -> Vec<&UpdateRecommendation> {
        recommendations
            .iter()
            .filter(|r| r.update_type == update_type)
            .collect()
    }

    /// Get only breaking updates (major version bumps)
    pub fn breaking_updates(
        recommendations: &[UpdateRecommendation],
    ) -> Vec<&UpdateRecommendation> {
        Self::filter_by_type(recommendations, UpdateType::Major)
    }

    /// Get only safe updates (minor/patch)
    pub fn safe_updates(recommendations: &[UpdateRecommendation]) -> Vec<&UpdateRecommendation> {
        recommendations
            .iter()
            .filter(|r| matches!(r.update_type, UpdateType::Minor | UpdateType::Patch))
            .collect()
    }
}

impl Default for UnifiedDependencyManager {
    fn default() -> Self {
        Self::new().expect("Failed to create UnifiedDependencyManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_by_type() {
        let recommendations = vec![
            UpdateRecommendation {
                package: "test1".to_string(),
                current_req: VersionReq {
                    raw: "1.0.0".to_string(),
                    ecosystem: Ecosystem::Rust,
                },
                current_installed: None,
                latest_version: "2.0.0".to_string(),
                update_type: UpdateType::Major,
                satisfies_requirement: false,
                registry: Registry::CratesIo,
                changelog_entries: vec![],
                releases: vec![],
            },
            UpdateRecommendation {
                package: "test2".to_string(),
                current_req: VersionReq {
                    raw: "1.0.0".to_string(),
                    ecosystem: Ecosystem::Rust,
                },
                current_installed: None,
                latest_version: "1.1.0".to_string(),
                update_type: UpdateType::Minor,
                satisfies_requirement: true,
                registry: Registry::CratesIo,
                changelog_entries: vec![],
                releases: vec![],
            },
        ];

        let major = UnifiedDependencyManager::filter_by_type(&recommendations, UpdateType::Major);
        assert_eq!(major.len(), 1);
        assert_eq!(major[0].package, "test1");

        let breaking = UnifiedDependencyManager::breaking_updates(&recommendations);
        assert_eq!(breaking.len(), 1);

        let safe = UnifiedDependencyManager::safe_updates(&recommendations);
        assert_eq!(safe.len(), 1);
        assert_eq!(safe[0].package, "test2");
    }
}
