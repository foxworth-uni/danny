use crate::security::{
    set_config_permissions, validate_project_id, validate_project_name, validate_project_path,
    SecurityError,
};
use crate::types::{DannyConfig, Project};
use chrono::{DateTime, Utc};
use danny_fs::{FileSystem, NativeFileSystem};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during config management
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    #[error("Config file not found at {0}")]
    ConfigNotFound(PathBuf),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Project already exists: {0}")]
    ProjectExists(String),

    #[error("Home directory not found")]
    HomeNotFound,
}

/// Manager for Danny configuration
///
/// Manages global Danny configuration stored in ~/.danny/config.toml.
/// For WASM builds, provide a FileSystem scoped to the config directory.
pub struct ConfigManager<F: FileSystem = NativeFileSystem> {
    fs: Arc<F>,
    config_path: PathBuf,
    config: DannyConfig,
}

impl ConfigManager {
    /// Get the default config path (~/.danny/config.toml)
    #[cfg(feature = "native-fs")]
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let home = dirs::home_dir().ok_or(ConfigError::HomeNotFound)?;
        Ok(home.join(".danny").join("config.toml"))
    }

    /// Load config from default location
    #[cfg(feature = "native-fs")]
    pub async fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        Self::load_from(&config_path).await
    }

    /// Load config from specific path (useful for testing)
    pub async fn load_from(path: &Path) -> Result<Self, ConfigError> {
        // Create FileSystem scoped to config directory
        let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let fs = Arc::new(NativeFileSystem::new(config_dir).map_err(ConfigError::Io)?);

        if !fs.exists(path).await.map_err(ConfigError::Io)? {
            return Err(ConfigError::ConfigNotFound(path.to_path_buf()));
        }

        let contents = fs.read_to_string(path).await.map_err(ConfigError::Io)?;
        let config: DannyConfig = toml::from_str(&contents)?;

        Ok(Self {
            fs,
            config_path: path.to_path_buf(),
            config,
        })
    }
}

impl<F: FileSystem> ConfigManager<F> {
    /// Load config with a custom FileSystem
    pub async fn load_with_filesystem(fs: Arc<F>, path: &Path) -> Result<Self, ConfigError> {
        if !fs.exists(path).await.map_err(ConfigError::Io)? {
            return Err(ConfigError::ConfigNotFound(path.to_path_buf()));
        }

        let contents = fs.read_to_string(path).await.map_err(ConfigError::Io)?;
        let config: DannyConfig = toml::from_str(&contents)?;

        Ok(Self {
            fs,
            config_path: path.to_path_buf(),
            config,
        })
    }
}

impl ConfigManager {
    /// Initialize a new config file
    #[cfg(feature = "native-fs")]
    pub async fn init() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        Self::init_at(&config_path).await
    }

    /// Initialize config at specific path
    pub async fn init_at(path: &Path) -> Result<Self, ConfigError> {
        // Create FileSystem scoped to config directory
        let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let fs = Arc::new(NativeFileSystem::new(config_dir).map_err(ConfigError::Io)?);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent).await.map_err(ConfigError::Io)?;
        }

        let config = DannyConfig::default();
        let toml_str = toml::to_string_pretty(&config)?;
        fs.write(path, &toml_str).await.map_err(ConfigError::Io)?;

        // Set restrictive permissions (sync operation, uses std::fs)
        set_config_permissions(path)?;

        Ok(Self {
            fs,
            config_path: path.to_path_buf(),
            config,
        })
    }

    /// Save config to disk atomically
    ///
    /// Uses a temporary file and atomic rename to prevent corruption
    pub async fn save(&self) -> Result<(), ConfigError> {
        let toml_str = toml::to_string_pretty(&self.config)?;

        // Write to temporary file first
        let temp_path = self.config_path.with_extension("toml.tmp");
        self.fs
            .write(&temp_path, &toml_str)
            .await
            .map_err(ConfigError::Io)?;

        // Set permissions on temp file (sync operation, uses std::fs)
        set_config_permissions(&temp_path)?;

        // Atomic rename
        self.fs
            .rename(&temp_path, &self.config_path)
            .await
            .map_err(ConfigError::Io)?;

        Ok(())
    }

    /// Get reference to config
    pub fn config(&self) -> &DannyConfig {
        &self.config
    }

    /// Get mutable reference to config (caller must call save())
    pub fn config_mut(&mut self) -> &mut DannyConfig {
        &mut self.config
    }

    /// Add a new project
    pub async fn add_project(&mut self, mut project: Project) -> Result<(), ConfigError> {
        // Validate project ID
        validate_project_id(&project.id)?;

        // Validate project name
        validate_project_name(&project.name)?;

        // Validate and canonicalize path
        let canonical_path = validate_project_path(&project.path)?;
        project.path = canonical_path;

        // Check for duplicate ID
        if self.config.projects.iter().any(|p| p.id == project.id) {
            return Err(ConfigError::ProjectExists(project.id));
        }

        self.config.projects.push(project);
        self.save().await?;

        Ok(())
    }

    /// Remove a project by ID
    pub async fn remove_project(&mut self, id: &str) -> Result<(), ConfigError> {
        let index = self
            .config
            .projects
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| ConfigError::ProjectNotFound(id.to_string()))?;

        self.config.projects.remove(index);
        self.save().await?;

        Ok(())
    }

    /// Enable a project
    pub async fn enable_project(&mut self, id: &str) -> Result<(), ConfigError> {
        let project = self
            .config
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ConfigError::ProjectNotFound(id.to_string()))?;

        project.enabled = true;
        self.save().await?;

        Ok(())
    }

    /// Disable a project
    pub async fn disable_project(&mut self, id: &str) -> Result<(), ConfigError> {
        let project = self
            .config
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ConfigError::ProjectNotFound(id.to_string()))?;

        project.enabled = false;
        self.save().await?;

        Ok(())
    }

    /// Update last checked timestamp for a project
    pub async fn update_last_checked(
        &mut self,
        id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<(), ConfigError> {
        let project = self
            .config
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ConfigError::ProjectNotFound(id.to_string()))?;

        project.last_checked = Some(timestamp);
        self.save().await?;

        Ok(())
    }

    /// Get a project by ID
    pub fn get_project(&self, id: &str) -> Option<&Project> {
        self.config.projects.iter().find(|p| p.id == id)
    }

    /// List all projects
    pub fn list_projects(&self) -> &[Project] {
        &self.config.projects
    }

    /// List enabled projects only
    pub fn list_enabled_projects(&self) -> Vec<&Project> {
        self.config.projects.iter().filter(|p| p.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    async fn create_test_manager() -> (ConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let manager = ConfigManager::init_at(&config_path).await.unwrap();
        (manager, temp_dir)
    }

    fn create_test_project(temp_dir: &TempDir, id: &str, name: &str) -> Project {
        let project_dir = temp_dir.path().join(id);
        fs::create_dir_all(&project_dir).unwrap();

        Project {
            id: id.to_string(),
            name: name.to_string(),
            path: project_dir,
            enabled: true,
            last_checked: None,
            settings: None,
            workspace_members: None,
        }
    }

    #[tokio::test]
    async fn test_init_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Init
        let manager = ConfigManager::init_at(&config_path).await.unwrap();
        assert_eq!(manager.config.version, "1.0");
        assert!(manager.config.projects.is_empty());

        // Load
        let loaded = ConfigManager::load_from(&config_path).await.unwrap();
        assert_eq!(loaded.config.version, "1.0");
    }

    #[tokio::test]
    async fn test_add_project() {
        let (mut manager, temp_dir) = create_test_manager().await;
        let project = create_test_project(&temp_dir, "test-project", "Test Project");

        manager.add_project(project.clone()).await.unwrap();
        assert_eq!(manager.config.projects.len(), 1);
        assert_eq!(manager.config.projects[0].id, "test-project");

        // Should persist
        let loaded = ConfigManager::load_from(&manager.config_path)
            .await
            .unwrap();
        assert_eq!(loaded.config.projects.len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_project_id() {
        let (mut manager, temp_dir) = create_test_manager().await;
        let project = create_test_project(&temp_dir, "test-project", "Test Project");

        manager.add_project(project.clone()).await.unwrap();
        let result = manager.add_project(project).await;
        assert!(matches!(result, Err(ConfigError::ProjectExists(_))));
    }

    #[tokio::test]
    async fn test_remove_project() {
        let (mut manager, temp_dir) = create_test_manager().await;
        let project = create_test_project(&temp_dir, "test-project", "Test Project");

        manager.add_project(project).await.unwrap();
        assert_eq!(manager.config.projects.len(), 1);

        manager.remove_project("test-project").await.unwrap();
        assert_eq!(manager.config.projects.len(), 0);
    }

    #[tokio::test]
    async fn test_enable_disable_project() {
        let (mut manager, temp_dir) = create_test_manager().await;
        let project = create_test_project(&temp_dir, "test-project", "Test Project");

        manager.add_project(project).await.unwrap();

        manager.disable_project("test-project").await.unwrap();
        assert!(!manager.config.projects[0].enabled);

        manager.enable_project("test-project").await.unwrap();
        assert!(manager.config.projects[0].enabled);
    }

    #[tokio::test]
    async fn test_update_last_checked() {
        let (mut manager, temp_dir) = create_test_manager().await;
        let project = create_test_project(&temp_dir, "test-project", "Test Project");

        manager.add_project(project).await.unwrap();

        let now = Utc::now();
        manager
            .update_last_checked("test-project", now)
            .await
            .unwrap();

        let project = manager.get_project("test-project").unwrap();
        assert!(project.last_checked.is_some());
        assert_eq!(project.last_checked.unwrap(), now);
    }

    #[tokio::test]
    async fn test_list_enabled_projects() {
        let (mut manager, temp_dir) = create_test_manager().await;

        let mut project1 = create_test_project(&temp_dir, "project1", "Project 1");
        project1.enabled = true;
        manager.add_project(project1).await.unwrap();

        let mut project2 = create_test_project(&temp_dir, "project2", "Project 2");
        project2.enabled = false;
        manager.add_project(project2).await.unwrap();

        let enabled = manager.list_enabled_projects();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "project1");
    }
}
