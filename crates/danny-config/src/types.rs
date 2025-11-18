use chrono::{DateTime, Utc};
use danny_core::{AnalysisCapabilities, AnalysisMode, Category, UnavailableReason};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure for Danny
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DannyConfig {
    /// Schema version for migrations
    pub version: String,

    /// Global settings
    pub settings: GlobalSettings,

    /// List of watched projects
    pub projects: Vec<Project>,
}

impl Default for DannyConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            projects: Vec::new(),
        }
    }
}

/// Global settings for Danny
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalSettings {
    /// Check interval in minutes
    #[serde(default = "default_check_interval")]
    pub check_interval_minutes: u32,

    /// Max concurrent project checks
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_checks: usize,

    /// Enable desktop notifications
    #[serde(default = "default_notifications")]
    pub enable_notifications: bool,

    /// Default feed configuration - fetch changelogs from GitHub
    #[serde(default)]
    pub fetch_changelogs: bool,

    /// Include dev dependencies in feed
    #[serde(default = "default_true")]
    pub include_dev_deps: bool,

    /// Only include dependencies from public registries
    #[serde(default = "default_true")]
    pub registry_only: bool,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            check_interval_minutes: default_check_interval(),
            max_concurrent_checks: default_max_concurrent(),
            enable_notifications: default_notifications(),
            fetch_changelogs: false,
            include_dev_deps: default_true(),
            registry_only: default_true(),
        }
    }
}

/// A watched project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    /// Unique identifier (UUID or slug)
    pub id: String,

    /// Display name
    pub name: String,

    /// Absolute path to project root
    pub path: PathBuf,

    /// Whether this project is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Last check timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<DateTime<Utc>>,

    /// Project-specific settings override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ProjectSettings>,

    /// Workspace member configuration (for monorepos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_members: Option<WorkspaceMemberConfig>,
}

/// Project-specific settings that override global settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSettings {
    /// Override global fetch_changelogs setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_changelogs: Option<bool>,

    /// Override global include_dev_deps setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_dev_deps: Option<bool>,

    /// Override global registry_only setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_only: Option<bool>,
}

/// Workspace member configuration for monorepos
///
/// Allows individual control over which workspace members to include in feed generation.
/// If None, all members are enabled (default behavior).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceMemberConfig {
    /// Specific members to enable (if None, all are enabled except disabled ones)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_members: Option<Vec<String>>,

    /// Specific members to disable (takes precedence over enabled_members)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_members: Option<Vec<String>>,
}

impl WorkspaceMemberConfig {
    /// Check if a workspace member should be included
    pub fn is_member_enabled(&self, member_name: &str) -> bool {
        // Disabled list takes precedence
        if let Some(disabled) = &self.disabled_members {
            if disabled.contains(&member_name.to_string()) {
                return false;
            }
        }

        // If enabled_members is specified, member must be in the list
        if let Some(enabled) = &self.enabled_members {
            return enabled.contains(&member_name.to_string());
        }

        // Default: enabled
        true
    }
}

// Default value functions
fn default_check_interval() -> u32 {
    60
}

fn default_max_concurrent() -> usize {
    5
}

fn default_notifications() -> bool {
    true
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_serialization() {
        let config = DannyConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            projects: vec![Project {
                id: "test-project".to_string(),
                name: "Test Project".to_string(),
                path: PathBuf::from("/tmp/test"),
                enabled: true,
                last_checked: None,
                settings: None,
                workspace_members: None,
            }],
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: DannyConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_default_config() {
        let config = DannyConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.settings.check_interval_minutes, 60);
        assert_eq!(config.settings.max_concurrent_checks, 5);
        assert!(config.settings.enable_notifications);
        assert!(config.projects.is_empty());
    }

    #[test]
    fn test_project_with_override_settings() {
        let project = Project {
            id: "test".to_string(),
            name: "Test".to_string(),
            path: PathBuf::from("/tmp/test"),
            enabled: true,
            last_checked: None,
            settings: Some(ProjectSettings {
                fetch_changelogs: Some(true),
                include_dev_deps: Some(false),
                registry_only: None,
            }),
            workspace_members: None,
        };

        let toml_str = toml::to_string(&project).unwrap();
        let deserialized: Project = toml::from_str(&toml_str).unwrap();
        assert_eq!(project, deserialized);
    }

    #[test]
    fn test_workspace_member_config() {
        // Test default behavior (all enabled)
        let config = WorkspaceMemberConfig {
            enabled_members: None,
            disabled_members: None,
        };
        assert!(config.is_member_enabled("danny-cli"));
        assert!(config.is_member_enabled("danny-feed"));

        // Test disabled list
        let config = WorkspaceMemberConfig {
            enabled_members: None,
            disabled_members: Some(vec!["danny-desktop".to_string()]),
        };
        assert!(config.is_member_enabled("danny-cli"));
        assert!(!config.is_member_enabled("danny-desktop"));

        // Test enabled list
        let config = WorkspaceMemberConfig {
            enabled_members: Some(vec!["danny-cli".to_string(), "danny-feed".to_string()]),
            disabled_members: None,
        };
        assert!(config.is_member_enabled("danny-cli"));
        assert!(config.is_member_enabled("danny-feed"));
        assert!(!config.is_member_enabled("danny-desktop"));

        // Test disabled takes precedence
        let config = WorkspaceMemberConfig {
            enabled_members: Some(vec!["danny-cli".to_string()]),
            disabled_members: Some(vec!["danny-cli".to_string()]),
        };
        assert!(!config.is_member_enabled("danny-cli"));
    }
}

/// What Danny should analyze - Package or Files mode
#[derive(Debug, Clone)]
pub enum AnalysisTarget {
    /// Full package analysis with complete dependency graph
    Package(PackageTarget),
    /// Limited file-level analysis
    Files(FilesTarget),
}

impl AnalysisTarget {
    /// Get the analysis capabilities for this target
    pub fn capabilities(&self) -> AnalysisCapabilities {
        match self {
            AnalysisTarget::Package(pkg) => pkg.capabilities(),
            AnalysisTarget::Files(files) => files.capabilities(),
        }
    }

    /// Get the analysis mode
    pub fn mode(&self) -> AnalysisMode {
        match self {
            AnalysisTarget::Package(_) => AnalysisMode::Package,
            AnalysisTarget::Files(_) => AnalysisMode::Files,
        }
    }

    /// Check if a category is supported
    pub fn supports_category(&self, category: Category) -> bool {
        self.capabilities().supports(category)
    }

    /// Get the root directory being analyzed
    pub fn root_dir(&self) -> &PathBuf {
        match self {
            AnalysisTarget::Package(pkg) => &pkg.root,
            AnalysisTarget::Files(files) => &files.working_dir,
        }
    }
}

/// Package analysis target (full capabilities)
#[derive(Debug, Clone)]
pub struct PackageTarget {
    /// Root directory containing package.json
    pub root: PathBuf,
    /// Entry points detected or configured
    pub entry_points: Vec<PathBuf>,
    /// Detected framework (if any)
    pub framework: Option<Framework>,
    /// Whether node_modules exists
    pub has_node_modules: bool,
}

impl PackageTarget {
    /// Get capabilities for package mode
    pub fn capabilities(&self) -> AnalysisCapabilities {
        let mut caps = AnalysisCapabilities::package_mode();

        // Add Dependencies if node_modules exists
        if self.has_node_modules {
            caps.mark_available(Category::Dependencies);
        } else {
            caps.mark_unavailable(
                Category::Dependencies,
                UnavailableReason::RequiresNodeModules,
            );
        }

        // Add Framework if detected
        if self.framework.is_some() {
            caps.mark_available(Category::Framework);
        } else {
            caps.mark_unavailable(
                Category::Framework,
                UnavailableReason::RequiresFrameworkDetection,
            );
        }

        caps
    }
}

/// Files analysis target (limited capabilities)
#[derive(Debug, Clone)]
pub struct FilesTarget {
    /// Specific files to analyze
    pub files: Vec<PathBuf>,
    /// Working directory for resolution
    pub working_dir: PathBuf,
    /// Nearby package.json found (for suggestions)
    pub nearby_package: Option<PathBuf>,
}

impl FilesTarget {
    /// Get capabilities for files mode (always limited)
    pub fn capabilities(&self) -> AnalysisCapabilities {
        AnalysisCapabilities::files_mode()
    }

    /// Suggest package mode if nearby package.json exists
    pub fn suggest_package_mode(&self) -> Option<PackageSuggestion> {
        self.nearby_package.as_ref().map(|pkg_path| {
            let pkg_dir = pkg_path.parent().unwrap_or(pkg_path);
            PackageSuggestion {
                package_json: pkg_path.clone(),
                command: format!("danny {}", pkg_dir.display()),
            }
        })
    }
}

/// Suggestion to use package mode instead
#[derive(Debug, Clone)]
pub struct PackageSuggestion {
    pub package_json: PathBuf,
    pub command: String,
}

/// Detected framework
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    NextJs,
    React,
    Vue,
    Svelte,
}

impl Framework {
    pub fn name(&self) -> &'static str {
        match self {
            Framework::NextJs => "Next.js",
            Framework::React => "React",
            Framework::Vue => "Vue",
            Framework::Svelte => "Svelte",
        }
    }
}
