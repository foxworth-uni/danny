//! Multi-source rule loading
//!
//! Loads TOML rules from multiple sources with proper priority handling.

use crate::constants::{
    MAX_DIRECTORY_DEPTH, MAX_REGEX_LENGTH, MAX_TOML_FILE_SIZE, REGEX_DFA_SIZE_LIMIT,
    REGEX_SIZE_LIMIT,
};
use crate::{Result, RuleError, TomlRule, TomlRuleFile};
use danny_fs::{DiscoveryOptions as FsDiscoveryOptions, FileSystem};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Loads rules from multiple sources
pub struct RuleLoader<F: FileSystem> {
    /// Filesystem abstraction for reading files
    fs: Arc<F>,

    /// Built-in rules (shipped with Danny)
    builtin_path: Option<PathBuf>,

    /// User rules (~/.config/danny/rules/)
    user_path: Option<PathBuf>,

    /// Project rules (.danny/rules/)
    project_path: Option<PathBuf>,
}

impl<F: FileSystem> RuleLoader<F> {
    /// Create a new rule loader with default paths
    pub fn new(fs: Arc<F>, project_root: &Path) -> Self {
        Self {
            fs,
            builtin_path: Self::find_builtin_rules(),
            user_path: Self::find_user_rules(),
            project_path: Some(project_root.join(".danny/rules")),
        }
    }

    /// Load all rules from all sources
    ///
    /// Priority order (higher overrides lower):
    /// 1. Built-in rules (lowest priority)
    /// 2. User rules
    /// 3. Project rules (highest priority)
    ///
    /// Note: Built-in and user rules are only loaded if they're within the FileSystem's
    /// project root. This ensures WASM compatibility where only project-local rules are available.
    pub async fn load_all(&self) -> Result<Vec<TomlRule>> {
        let mut all_rules = Vec::new();
        let project_root = self.fs.project_root();

        // Load built-in rules (only if within project root)
        // Try to normalize the path - if it fails (outside project root), skip it
        if let Some(ref path) = self.builtin_path {
            match self.fs.normalize_path(path).await {
                Ok(normalized) => {
                    if normalized.starts_with(project_root)
                        && self
                            .fs
                            .exists(&normalized)
                            .await
                            .map_err(RuleError::IoError)?
                    {
                        all_rules.extend(self.load_from_directory(&normalized).await?);
                    }
                }
                Err(_) => {
                    // Path is outside project root (e.g., system-wide rules in WASM) - skip
                }
            }
        }

        // Load user rules (only if within project root)
        // Try to normalize the path - if it fails (outside project root), skip it
        if let Some(ref path) = self.user_path {
            match self.fs.normalize_path(path).await {
                Ok(normalized) => {
                    if normalized.starts_with(project_root)
                        && self
                            .fs
                            .exists(&normalized)
                            .await
                            .map_err(RuleError::IoError)?
                    {
                        all_rules.extend(self.load_from_directory(&normalized).await?);
                    }
                }
                Err(_) => {
                    // Path is outside project root (e.g., user config dir in WASM) - skip
                }
            }
        }

        // Load project rules (always within project root)
        if let Some(ref path) = self.project_path {
            if self.fs.exists(path).await.map_err(RuleError::IoError)? {
                all_rules.extend(self.load_from_directory(path).await?);
            }
        }

        // Sort by priority (higher priority first), then by name for stable ordering
        // Using sort_by instead of sort_by_key ensures deterministic behavior
        all_rules.sort_by(|a, b| {
            b.priority
                .unwrap_or(0)
                .cmp(&a.priority.unwrap_or(0))
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(all_rules)
    }

    /// Load rules from a specific directory
    async fn load_from_directory(&self, dir: &Path) -> Result<Vec<TomlRule>> {
        let mut rules = Vec::new();

        // Normalize the directory path for security checks
        let normalized_dir =
            self.fs
                .normalize_path(dir)
                .await
                .map_err(|e| RuleError::LoadError {
                    path: dir.display().to_string(),
                    source: Box::new(e),
                })?;

        // Discover .toml files in the directory
        let discovery_options = FsDiscoveryOptions {
            max_file_size: Some(MAX_TOML_FILE_SIZE),
            follow_symlinks: false, // Security: don't follow symlinks
            max_depth: MAX_DIRECTORY_DEPTH,
            include_hidden: false,
            respect_gitignore: false, // Don't ignore .toml files
        };

        // Use "toml" extension (without dot) for discovery
        let discovered_files = self
            .fs
            .discover_files(&normalized_dir, &["toml"], &[], &discovery_options)
            .await
            .map_err(|e| RuleError::LoadError {
                path: normalized_dir.display().to_string(),
                source: Box::new(e),
            })?;

        // Load each discovered TOML file
        for path in discovered_files {
            // Security: Verify path is within the expected directory
            // (FileSystem already validates this, but double-check for safety)
            let normalized_path =
                self.fs
                    .normalize_path(&path)
                    .await
                    .map_err(|e| RuleError::LoadError {
                        path: path.display().to_string(),
                        source: Box::new(e),
                    })?;

            if !normalized_path.starts_with(&normalized_dir) {
                continue; // Skip paths outside the expected directory
            }

            // Security: Check file size before reading
            let metadata = self
                .fs
                .metadata(&path)
                .await
                .map_err(|e| RuleError::LoadError {
                    path: path.display().to_string(),
                    source: Box::new(e),
                })?;

            if metadata.size > MAX_TOML_FILE_SIZE {
                return Err(RuleError::LoadError {
                    path: path.display().to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "File exceeds maximum size of {}MB",
                            MAX_TOML_FILE_SIZE / 1_048_576
                        ),
                    )),
                });
            }

            // Load and parse the file
            let file_rules = self.load_from_file(&path).await?;
            rules.extend(file_rules);
        }

        Ok(rules)
    }

    /// Load rules from a single TOML file
    async fn load_from_file(&self, path: &Path) -> Result<Vec<TomlRule>> {
        let contents = self
            .fs
            .read_to_string(path)
            .await
            .map_err(|e| RuleError::LoadError {
                path: path.display().to_string(),
                source: Box::new(e),
            })?;

        let file: TomlRuleFile = toml::from_str(&contents).map_err(|e| RuleError::LoadError {
            path: path.display().to_string(),
            source: Box::new(e),
        })?;

        // Security: Validate regex patterns to prevent ReDoS
        for rule in &file.rules {
            if let Some(ref pattern) = rule.matcher.export_pattern {
                validate_regex_pattern(pattern, path)?;
            }
            if let Some(ref pattern) = rule.matcher.path_pattern {
                validate_regex_pattern(pattern, path)?;
            }
        }

        Ok(file.rules)
    }

    /// Find built-in rules directory
    ///
    /// Uses a multi-strategy approach to locate rules:
    /// 1. Check DANNY_RULES_DIR environment variable (highest priority)
    /// 2. Check paths relative to executable (for dev builds)
    /// 3. Check system paths (Unix/Windows standard locations)
    /// 4. Fallback: Built-in rules are embedded via include_str!() in built_in.rs
    fn find_builtin_rules() -> Option<PathBuf> {
        // Strategy 1: Check environment variable (highest priority)
        if let Ok(env_path) = std::env::var("DANNY_RULES_DIR") {
            let path = PathBuf::from(env_path);
            if path.exists() && path.is_dir() {
                return Some(path);
            }
        }

        // Strategy 2: Check paths relative to executable (for dev builds)
        if let Ok(exe) = std::env::current_exe() {
            // Try ../rules relative to executable
            if let Some(exe_dir) = exe.parent() {
                let relative_path = exe_dir.join("../rules");
                if relative_path.exists() && relative_path.is_dir() {
                    return Some(relative_path);
                }
                // Try ../share/danny/rules (common installation layout)
                let share_path = exe_dir.join("../share/danny/rules");
                if share_path.exists() && share_path.is_dir() {
                    return Some(share_path);
                }
            }
        }

        // Strategy 3: Check system paths (Unix/Windows standard locations)
        #[cfg(unix)]
        {
            // Unix: /usr/local/share/danny/rules, /usr/share/danny/rules
            let unix_paths = [
                PathBuf::from("/usr/local/share/danny/rules"),
                PathBuf::from("/usr/share/danny/rules"),
            ];
            for path in unix_paths.iter() {
                if path.exists() && path.is_dir() {
                    return Some(path.clone());
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows: %PROGRAMDATA%\danny\rules, %APPDATA%\danny\rules
            if let Some(program_data) = std::env::var_os("PROGRAMDATA") {
                let path = PathBuf::from(program_data).join("danny/rules");
                if path.exists() && path.is_dir() {
                    return Some(path);
                }
            }
            if let Some(app_data) = std::env::var_os("APPDATA") {
                let path = PathBuf::from(app_data).join("danny/rules");
                if path.exists() && path.is_dir() {
                    return Some(path);
                }
            }
        }

        // Strategy 4: Fallback - Built-in rules are embedded via include_str!() in built_in.rs
        // Return None to indicate we should use embedded rules
        None
    }

    /// Find user rules directory
    fn find_user_rules() -> Option<PathBuf> {
        #[cfg(feature = "native-fs")]
        {
            dirs::config_dir().map(|config| config.join("danny/rules"))
        }
        #[cfg(not(feature = "native-fs"))]
        {
            // WASM: User rules not supported (no filesystem access)
            None
        }
    }
}

/// Validates regex patterns to prevent ReDoS attacks
///
/// Note: This function performs basic validation during rule loading.
/// More comprehensive protection (size limits, DFA limits) is applied
/// during regex compilation in CompiledMatcher::from_toml().
fn validate_regex_pattern(pattern: &str, path: &Path) -> Result<()> {
    if pattern.len() > MAX_REGEX_LENGTH {
        return Err(RuleError::LoadError {
            path: path.display().to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Regex pattern exceeds {} characters: {}",
                    MAX_REGEX_LENGTH, pattern
                ),
            )),
        });
    }

    // Basic syntax validation - detailed protection happens during compilation
    regex::RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
        .map_err(|e| RuleError::LoadError {
            path: path.display().to_string(),
            source: Box::new(e),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use danny_fs::NativeFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let rule_file = temp_dir.path().join("test.toml");

        std::fs::write(
            &rule_file,
            r#"
[[rules]]
name = "test-rule"

[rules.match]
export_pattern = "^test"

[rules.action]
mark_used = true
"#,
        )
        .unwrap();

        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());
        let loader = RuleLoader {
            fs,
            builtin_path: None,
            user_path: None,
            project_path: Some(temp_dir.path().to_path_buf()),
        };

        let rules = loader.load_from_file(&rule_file).await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "test-rule");
    }
}
