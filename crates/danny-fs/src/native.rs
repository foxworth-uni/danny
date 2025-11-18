//! Native filesystem implementation using std::fs + tokio.

use crate::{FileSystem, FileMetadata, DiscoveryOptions};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::io;
use tokio::task;

#[cfg(feature = "native")]
use ignore::WalkBuilder;

/// Native filesystem implementation using std::fs + tokio.
///
/// This implementation wraps blocking std::fs calls with tokio::spawn_blocking
/// to avoid blocking the async runtime.
#[derive(Debug, Clone)]
pub struct NativeFileSystem {
    project_root: PathBuf,
    canonical_root: PathBuf,
}

impl NativeFileSystem {
    /// Create a new native filesystem scoped to a project root.
    ///
    /// # Errors
    ///
    /// Returns an error if the root doesn't exist or can't be canonicalized.
    pub fn new(project_root: impl AsRef<Path>) -> io::Result<Self> {
        let project_root = project_root.as_ref()
            .canonicalize()
            .or_else(|_| {
                // If canonicalize fails (e.g., path doesn't exist yet),
                // try to canonicalize the parent and join the last component
                if let Some(parent) = project_root.as_ref().parent() {
                    let name = project_root.as_ref().file_name()
                        .ok_or_else(|| io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Invalid project root path"
                        ))?;
                    Ok(parent.canonicalize()?.join(name))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Project root does not exist: {}", project_root.as_ref().display())
                    ))
                }
            })?;

        let canonical_root = project_root.canonicalize()
            .unwrap_or_else(|_| project_root.clone());

        Ok(Self { 
            project_root: project_root.clone(),
            canonical_root,
        })
    }

    /// Validate that a path is within the project root.
    ///
    /// # Security
    ///
    /// This prevents path traversal attacks by ensuring operations
    /// can't escape the project directory.
    fn validate_path(&self, path: &Path) -> io::Result<PathBuf> {
        // Make path absolute if it's relative
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        // Always try to canonicalize for symlink resolution
        // If the path doesn't exist yet, canonicalize the parent and join
        let canonical_path = match absolute.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                // Path doesn't exist - canonicalize parent and join filename
                if let Some(parent) = absolute.parent() {
                    if let Some(file_name) = absolute.file_name() {
                        match parent.canonicalize() {
                            Ok(canonical_parent) => canonical_parent.join(file_name),
                            Err(_) => {
                                // Parent doesn't exist - do syntactic normalization
                                // and check if it would escape root
                                let normalized = self.normalize_path_sync(&absolute)?;
                                // Check if normalized path would be outside root
                                if !normalized.starts_with(&self.canonical_root) {
                                    return Err(io::Error::new(
                                        io::ErrorKind::PermissionDenied,
                                        format!(
                                            "Path traversal detected: {} is outside project root {}",
                                            normalized.display(),
                                            self.project_root.display()
                                        )
                                    ));
                                }
                                normalized
                            }
                        }
                    } else {
                        // No filename - just use absolute
                        absolute.clone()
                    }
                } else {
                    absolute.clone()
                }
            }
        };

        // Check it's within project root (compare canonicalized paths)
        if !canonical_path.starts_with(&self.canonical_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Path traversal detected: {} is outside project root {}",
                    canonical_path.display(),
                    self.project_root.display()
                )
            ));
        }

        Ok(canonical_path)
    }

    /// Synchronous path normalization for internal use.
    fn normalize_path_sync(&self, path: &Path) -> io::Result<PathBuf> {
        // Try canonicalize first (best option if path exists)
        if let Ok(canonical) = path.canonicalize() {
            return Ok(canonical);
        }

        // If path doesn't exist, do syntactic normalization
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                    components.clear();
                    components.push(component.as_os_str().to_owned());
                }
                std::path::Component::CurDir => {
                    // Skip "."
                }
                std::path::Component::ParentDir => {
                    // Go up one level
                    if components.len() > 1 {
                        components.pop();
                    }
                }
                std::path::Component::Normal(name) => {
                    components.push(name.to_owned());
                }
            }
        }

        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }
        Ok(result)
    }
}

#[async_trait::async_trait]
impl FileSystem for NativeFileSystem {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || Ok(validated.exists()))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || std::fs::read_to_string(&validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || std::fs::read(&validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || {
            match std::fs::symlink_metadata(&validated) {
                Ok(meta) => Ok(FileMetadata {
                    exists: true,
                    is_file: meta.is_file(),
                    is_dir: meta.is_dir(),
                    is_symlink: meta.file_type().is_symlink(),
                    size: meta.len(),
                }),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    Ok(FileMetadata {
                        exists: false,
                        is_file: false,
                        is_dir: false,
                        is_symlink: false,
                        size: 0,
                    })
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        let validated = self.validate_path(path)?;
        let contents = contents.to_string();
        task::spawn_blocking(move || std::fs::write(&validated, contents))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn write_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let validated = self.validate_path(path)?;
        let contents = contents.to_vec();
        task::spawn_blocking(move || std::fs::write(&validated, contents))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        let validated = self.validate_path(path)?;

        // Security: Check it's not a symlink (unless explicitly allowed elsewhere)
        let meta = self.metadata(path).await?;
        if meta.is_symlink {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Refusing to remove symlink (use explicit symlink removal method)"
            ));
        }

        task::spawn_blocking(move || std::fs::remove_file(&validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let from_validated = self.validate_path(from)?;
        let to_validated = self.validate_path(to)?;
        task::spawn_blocking(move || std::fs::rename(&from_validated, &to_validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || std::fs::create_dir(&validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let validated = self.validate_path(path)?;
        task::spawn_blocking(move || std::fs::create_dir_all(&validated))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn discover_files(
        &self,
        root: &Path,
        extensions: &[&str],
        ignore_patterns: &[&str],
        options: &DiscoveryOptions,
    ) -> io::Result<HashSet<PathBuf>> {
        let validated_root = self.validate_path(root)?;
        let extensions: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();
        let ignore_patterns: Vec<String> = ignore_patterns.iter().map(|s| s.to_string()).collect();
        let opts = options.clone();
        let canonical_root = self.canonical_root.clone();

        task::spawn_blocking(move || {
            discover_files_sync(&validated_root, &extensions, &ignore_patterns, &opts, &canonical_root)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn normalize_path(&self, path: &Path) -> io::Result<PathBuf> {
        let path_buf = path.to_path_buf();
        let canonical_root = self.canonical_root.clone();
        task::spawn_blocking(move || {
            let normalized = if path_buf.is_absolute() {
                path_buf.canonicalize().unwrap_or(path_buf)
            } else {
                canonical_root.join(&path_buf).canonicalize()
                    .unwrap_or_else(|_| canonical_root.join(&path_buf))
            };

            // Validate against project root
            if !normalized.starts_with(&canonical_root) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Path outside project root"
                ));
            }

            Ok(normalized)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    fn project_root(&self) -> &Path {
        &self.project_root
    }
}

/// Synchronous file discovery implementation.
#[cfg(feature = "native")]
fn discover_files_sync(
    root: &Path,
    extensions: &[String],
    ignore_patterns: &[String],
    options: &DiscoveryOptions,
    project_root: &Path,
) -> io::Result<HashSet<PathBuf>> {
    let mut discovered = HashSet::new();

    let mut walker = WalkBuilder::new(root);
    walker
        .follow_links(options.follow_symlinks)
        .hidden(!options.include_hidden)
        .git_ignore(options.respect_gitignore)
        .git_exclude(options.respect_gitignore)
        .max_depth(Some(options.max_depth))
        .max_filesize(options.max_file_size);

    // Add custom ignore patterns
    if !ignore_patterns.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(root);
        for pattern in ignore_patterns {
            overrides
                .add(&format!("!{}", pattern))
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        }
        if let Ok(ov) = overrides.build() {
            walker.overrides(ov);
        }
    }

    for result in walker.build() {
        let entry = result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        let path = entry.path();

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_with_dot = format!(".{}", ext);
            if extensions.contains(&ext_with_dot) {
                // Ensure it's within project root (security)
                if let Ok(canonical) = path.canonicalize() {
                    if canonical.starts_with(project_root) {
                        discovered.insert(canonical);
                    }
                }
            }
        }
    }

    Ok(discovered)
}

