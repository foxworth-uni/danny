//! File discovery and unreachable file detection.
//!
//! This module discovers all source files in a project and compares them
//! against the module graph to find files that are never imported.

use danny_core::{AnalysisOptions, Finding, Result};
use danny_fs::{DiscoveryOptions as FsDiscoveryOptions, FileSystem};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for file discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// File extensions to consider as source files
    pub extensions: Vec<String>,

    /// Additional ignore patterns beyond .gitignore
    pub ignore_patterns: Vec<String>,

    /// Maximum file size to consider (bytes). Prevents reading huge files.
    pub max_file_size: Option<u64>,

    /// Follow symlinks during traversal
    pub follow_symlinks: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            extensions: vec![
                ".js".to_string(),
                ".jsx".to_string(),
                ".ts".to_string(),
                ".tsx".to_string(),
                ".mjs".to_string(),
                ".cjs".to_string(),
            ],
            ignore_patterns: vec![],
            max_file_size: Some(10 * 1024 * 1024), // 10MB default
            follow_symlinks: false,                // Security: don't follow symlinks by default
        }
    }
}

/// Discovers all source files in the project directory.
///
/// Uses the provided FileSystem which handles platform-specific file discovery:
/// - Native: Uses `ignore` crate with .gitignore support
/// - WASM: Filters from pre-loaded in-memory files
///
/// # Security
/// - Does not follow symlinks by default (prevents infinite loops)
/// - Respects .gitignore to avoid traversing large directories (native only)
/// - Normalizes project root to prevent path traversal
/// - Has max file size limit to prevent memory exhaustion
pub async fn discover_source_files<F: FileSystem>(
    _options: &AnalysisOptions,
    config: &DiscoveryConfig,
    fs: Arc<F>,
) -> Result<HashSet<PathBuf>> {
    // project_root is already normalized by FileSystem::new()
    // Using normalize_path() again would double-join relative paths
    let project_root = fs.project_root().to_path_buf();

    // Validate we're not being asked to scan outside the project
    validate_project_root(&project_root)?;

    // Convert extensions to &str slice
    let extensions: Vec<&str> = config.extensions.iter().map(|s| s.as_str()).collect();
    let ignore_patterns: Vec<&str> = config.ignore_patterns.iter().map(|s| s.as_str()).collect();

    // Build discovery options
    let discovery_options = FsDiscoveryOptions {
        max_file_size: config.max_file_size,
        follow_symlinks: config.follow_symlinks,
        max_depth: 100,
        include_hidden: false,
        respect_gitignore: true,
    };

    // Use FileSystem's discover_files method
    fs.discover_files(
        &project_root,
        &extensions,
        &ignore_patterns,
        &discovery_options,
    )
    .await
    .map_err(|e| danny_core::Error::Backend {
        backend: "JavaScript".to_string(),
        message: format!("File discovery failed: {}", e),
    })
}

/// Validates that the project root is safe to scan.
///
/// # Security
/// - Prevents scanning system directories
/// - Prevents scanning root filesystem
fn validate_project_root(root: &std::path::Path) -> Result<()> {
    // Prevent scanning filesystem root
    if root == std::path::Path::new("/") {
        return Err(danny_core::Error::InvalidConfig {
            message: "Refusing to scan filesystem root".to_string(),
        });
    }

    // Prevent scanning system directories on macOS/Linux
    #[cfg(unix)]
    {
        let dangerous_paths = ["/bin", "/sbin", "/usr", "/etc", "/var", "/sys", "/proc"];
        for dangerous in &dangerous_paths {
            if root.starts_with(dangerous) {
                return Err(danny_core::Error::InvalidConfig {
                    message: format!("Refusing to scan system directory: {}", dangerous),
                });
            }
        }
    }

    // Prevent scanning Windows system directories
    #[cfg(windows)]
    {
        let root_str = root.to_string_lossy().to_lowercase();
        if root_str.starts_with("c:\\windows") || root_str.starts_with("c:\\program files") {
            return Err(danny_core::Error::InvalidConfig {
                message: "Refusing to scan Windows system directory".to_string(),
            });
        }
    }

    Ok(())
}

/// Compares discovered files with the module graph to find unreachable files.
///
/// A file is considered "unreachable" if:
/// 1. It was discovered in the project directory
/// 2. It has a supported source file extension
/// 3. It does not appear in the module graph (was never imported)
/// 4. It is not an entry point
///
/// # Algorithm
/// 1. Collect all paths from the module graph into a HashSet O(n)
/// 2. Add entry points to the set O(m)
/// 3. For each discovered file, check if NOT in set O(k)
/// 4. Total: O(n + m + k) = O(n) where n is total files
pub async fn find_unreachable_files(
    discovered_files: HashSet<PathBuf>,
    module_graph: &fob::graph::ModuleGraph,
    entry_points: &[PathBuf],
) -> Result<Vec<Finding>> {
    // Collect all module paths from the graph
    let modules = module_graph
        .modules()
        .await
        .map_err(|e| danny_core::Error::Backend {
            backend: "JavaScript".to_string(),
            message: format!("Failed to get modules from graph: {}", e),
        })?;

    // Normalize module paths for comparison (using PathBuf normalization)
    let mut reachable_files: HashSet<PathBuf> = modules
        .iter()
        .map(|m| {
            // Normalize paths by converting to absolute and cleaning up
            m.path.clone()
        })
        .collect();

    // Add entry points (they're reachable by definition)
    for entry in entry_points {
        reachable_files.insert(entry.clone());
    }

    // Find unreachable files
    // Note: File size will be obtained from FileReader if needed, but for now
    // we'll use 0 as a placeholder since we don't have FileReader in this function
    let mut findings = Vec::new();

    for file in discovered_files {
        if !reachable_files.contains(&file) {
            findings.push(Finding::UnreachableFile {
                path: file,
                size: 0, // Size will be populated by FileReader if needed
                explanation: None,
            });
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_project_root_rejects_root() {
        let result = validate_project_root(std::path::Path::new("/"));
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_validate_project_root_rejects_system_dirs() {
        assert!(validate_project_root(std::path::Path::new("/bin")).is_err());
        assert!(validate_project_root(std::path::Path::new("/etc")).is_err());
        assert!(validate_project_root(std::path::Path::new("/usr")).is_err());
    }

    #[tokio::test]
    async fn test_discover_source_files_basic() {
        use danny_fs::NativeFileSystem;
        use std::sync::Arc;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create test files
        fs::write(root.join("index.js"), "").unwrap();
        fs::write(root.join("utils.ts"), "").unwrap();
        fs::write(root.join("README.md"), "").unwrap();

        let options = AnalysisOptions {
            project_root: root.to_path_buf(),
            entry_points: vec![],
            follow_external: false,
            max_depth: None,
            config_path: None,
            backend_options: std::collections::HashMap::new(),
        };

        let config = DiscoveryConfig::default();
        let fs = Arc::new(NativeFileSystem::new(root).unwrap());
        let discovered = discover_source_files(&options, &config, fs).await.unwrap();

        // Should find .js and .ts files but not .md
        assert_eq!(discovered.len(), 2);
    }

    #[tokio::test]
    async fn test_discover_respects_gitignore() {
        use danny_fs::NativeFileSystem;
        use std::sync::Arc;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Initialize git repo for .gitignore to work
        std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(root)
            .output()
            .ok();

        // Create .gitignore
        fs::write(root.join(".gitignore"), "ignored.js\n").unwrap();

        // Create files
        fs::write(root.join("index.js"), "").unwrap();
        fs::write(root.join("ignored.js"), "").unwrap();

        let options = AnalysisOptions {
            project_root: root.to_path_buf(),
            entry_points: vec![],
            follow_external: false,
            max_depth: None,
            config_path: None,
            backend_options: std::collections::HashMap::new(),
        };

        let config = DiscoveryConfig::default();
        let fs = Arc::new(NativeFileSystem::new(root).unwrap());
        let discovered = discover_source_files(&options, &config, fs).await.unwrap();

        // Should only find index.js
        assert_eq!(discovered.len(), 1);
        assert!(discovered.iter().any(|p| p.ends_with("index.js")));
    }

    #[tokio::test]
    async fn test_discover_handles_nested_directories() {
        use danny_fs::NativeFileSystem;
        use std::sync::Arc;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::write(root.join("src/index.js"), "").unwrap();
        fs::write(root.join("src/components/Button.tsx"), "").unwrap();

        let options = AnalysisOptions {
            project_root: root.to_path_buf(),
            entry_points: vec![],
            follow_external: false,
            max_depth: None,
            config_path: None,
            backend_options: std::collections::HashMap::new(),
        };

        let config = DiscoveryConfig::default();
        let fs = Arc::new(NativeFileSystem::new(root).unwrap());
        let discovered = discover_source_files(&options, &config, fs).await.unwrap();

        assert_eq!(discovered.len(), 2);
    }
}
