//! WASM filesystem implementation using in-memory storage.

use crate::{DiscoveryOptions, FileMetadata, FileSystem};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "wasm")]
use parking_lot::RwLock;

#[cfg(feature = "wasm")]
use std::sync::Arc;

/// WASM filesystem implementation using in-memory storage.
///
/// Files are pre-loaded from the JavaScript host via FFI and stored in memory.
/// This enables Danny to work in browser and edge runtimes without filesystem access.
///
/// # Thread Safety
///
/// Uses `Arc<RwLock<HashMap>>` for interior mutability:
/// - Multiple concurrent readers (common case)
/// - Exclusive writer (rare: only during setup)
#[derive(Clone)]
pub struct WasmFileSystem {
    project_root: PathBuf,
    #[cfg(feature = "wasm")]
    files: Arc<RwLock<HashMap<PathBuf, Vec<u8>>>>,
}

impl WasmFileSystem {
    /// Create a new WASM filesystem from pre-loaded files.
    ///
    /// # Parameters
    ///
    /// - `project_root`: Virtual project root (e.g., "/project")
    /// - `files`: Map of absolute paths to file contents
    ///
    /// # Example (from WASM bindings)
    ///
    /// ```rust,ignore
    /// #[wasm_bindgen]
    /// pub fn create_analyzer(files: JsValue) -> Result<Analyzer, JsValue> {
    ///     let files_map: HashMap<String, String> = serde_wasm_bindgen::from_value(files)?;
    ///     let files: HashMap<PathBuf, Vec<u8>> = files_map
    ///         .into_iter()
    ///         .map(|(k, v)| (PathBuf::from(k), v.into_bytes()))
    ///         .collect();
    ///
    ///     let fs = WasmFileSystem::new("/project", files)?;
    ///     Ok(Analyzer::new(fs))
    /// }
    /// ```
    pub fn new(
        project_root: impl AsRef<Path>,
        files: HashMap<PathBuf, Vec<u8>>,
    ) -> io::Result<Self> {
        let project_root = Self::normalize_path_sync(project_root.as_ref())?;

        // Validate all file paths are within project root
        for path in files.keys() {
            let normalized = Self::normalize_path_sync(path)?;
            if !normalized.starts_with(&project_root) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("File path outside project root: {}", path.display()),
                ));
            }
        }

        Ok(Self {
            project_root,
            #[cfg(feature = "wasm")]
            files: Arc::new(RwLock::new(files)),
        })
    }

    /// Create an empty WASM filesystem (useful for testing).
    pub fn empty(project_root: impl AsRef<Path>) -> io::Result<Self> {
        Self::new(project_root, HashMap::new())
    }

    /// Add a file to the in-memory filesystem (used during setup).
    pub fn add_file(&self, path: PathBuf, contents: Vec<u8>) -> io::Result<()> {
        let normalized = Self::normalize_path_sync(&path)?;
        if !normalized.starts_with(&self.project_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Path outside project root",
            ));
        }

        #[cfg(feature = "wasm")]
        {
            self.files.write().insert(normalized, contents);
        }
        Ok(())
    }

    /// Synchronous path normalization (WASM doesn't have async I/O).
    fn normalize_path_sync(path: &Path) -> io::Result<PathBuf> {
        let mut components = Vec::new();
        let mut is_absolute = false;

        for component in path.components() {
            match component {
                std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                    is_absolute = true;
                    components.clear();
                }
                std::path::Component::CurDir => {
                    // Skip "."
                }
                std::path::Component::ParentDir => {
                    // Security: Reject paths that try to escape root
                    // For absolute paths with <= 1 component, reject ..
                    // For relative paths with 0 components, reject ..
                    if components.is_empty() || (components.len() == 1 && is_absolute) {
                        return Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "Path attempts to escape project root using ..",
                        ));
                    }
                    components.pop();
                }
                std::path::Component::Normal(name) => {
                    components.push(name);
                }
            }
        }

        let mut result = PathBuf::new();
        if is_absolute {
            result.push("/");
        }
        for component in components {
            result.push(component);
        }

        Ok(result)
    }

    /// Validate path against project root (security).
    fn validate_path(&self, path: &Path) -> io::Result<PathBuf> {
        let normalized = Self::normalize_path_sync(path)?;

        if !normalized.starts_with(&self.project_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Path traversal detected: {} is outside {}",
                    normalized.display(),
                    self.project_root.display()
                ),
            ));
        }

        Ok(normalized)
    }
}

#[async_trait::async_trait]
impl FileSystem for WasmFileSystem {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        let normalized = self.validate_path(path)?;
        #[cfg(feature = "wasm")]
        {
            Ok(self.files.read().contains_key(&normalized))
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = normalized;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path).await?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let normalized = self.validate_path(path)?;
        #[cfg(feature = "wasm")]
        {
            self.files.read().get(&normalized).cloned().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File not found: {}", normalized.display()),
                )
            })
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = normalized;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let normalized = self.validate_path(path)?;
        #[cfg(feature = "wasm")]
        {
            let files = self.files.read();

            match files.get(&normalized) {
                Some(contents) => Ok(FileMetadata {
                    exists: true,
                    is_file: true,
                    is_dir: false,
                    is_symlink: false,
                    size: contents.len() as u64,
                }),
                None => Ok(FileMetadata {
                    exists: false,
                    is_file: false,
                    is_dir: false,
                    is_symlink: false,
                    size: 0,
                }),
            }
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = normalized;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.write_bytes(path, contents.as_bytes()).await
    }

    async fn write_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let normalized = self.validate_path(path)?;
        #[cfg(feature = "wasm")]
        {
            self.files.write().insert(normalized, contents.to_vec());
            Ok(())
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = (normalized, contents);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        let normalized = self.validate_path(path)?;
        #[cfg(feature = "wasm")]
        {
            self.files
                .write()
                .remove(&normalized)
                .map(|_| ())
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not found"))
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = normalized;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let from_normalized = self.validate_path(from)?;
        let to_normalized = self.validate_path(to)?;

        #[cfg(feature = "wasm")]
        {
            let mut files = self.files.write();
            let contents = files
                .remove(&from_normalized)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Source file not found"))?;

            files.insert(to_normalized, contents);
            Ok(())
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = (from_normalized, to_normalized);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn create_dir(&self, _path: &Path) -> io::Result<()> {
        // WASM filesystem is flat - directories are implicit
        Ok(())
    }

    async fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        // WASM filesystem is flat - directories are implicit
        Ok(())
    }

    async fn discover_files(
        &self,
        root: &Path,
        extensions: &[&str],
        _ignore_patterns: &[&str],
        _options: &DiscoveryOptions,
    ) -> io::Result<HashSet<PathBuf>> {
        let normalized_root = self.validate_path(root)?;
        let root_str = normalized_root.to_string_lossy();

        #[cfg(feature = "wasm")]
        {
            let files = self.files.read();
            let mut discovered = HashSet::new();

            for (path, _) in files.iter() {
                let path_str = path.to_string_lossy();

                // Check if within root
                if !path_str.starts_with(root_str.as_ref()) {
                    continue;
                }

                // Check extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_with_dot = format!(".{}", ext);
                    if extensions.iter().any(|e| *e == ext_with_dot) {
                        discovered.insert(path.clone());
                    }
                }
            }

            // Note: ignore_patterns and options are not fully implemented for WASM
            // The JavaScript host should pre-filter files before passing to WASM

            Ok(discovered)
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = (normalized_root, root_str, extensions);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "WASM feature not enabled",
            ))
        }
    }

    async fn normalize_path(&self, path: &Path) -> io::Result<PathBuf> {
        Self::normalize_path_sync(path)
    }

    fn project_root(&self) -> &Path {
        &self.project_root
    }
}
