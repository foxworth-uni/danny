//! FileSystem trait for platform-agnostic filesystem operations.

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::io;

/// File metadata compatible across platforms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// Whether the path exists.
    pub exists: bool,
    /// Whether the path is a file (false if directory or doesn't exist).
    pub is_file: bool,
    /// Whether the path is a directory.
    pub is_dir: bool,
    /// Whether the path is a symbolic link.
    pub is_symlink: bool,
    /// File size in bytes (0 for directories or non-existent files).
    pub size: u64,
}

/// Options for file discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Maximum file size to include (bytes).
    pub max_file_size: Option<u64>,

    /// Follow symbolic links (default: false for security).
    pub follow_symlinks: bool,

    /// Maximum directory depth (default: 100).
    pub max_depth: usize,

    /// Include hidden files (default: false).
    pub include_hidden: bool,

    /// Respect .gitignore files (default: true).
    pub respect_gitignore: bool,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            max_file_size: Some(10 * 1024 * 1024), // 10MB default
            follow_symlinks: false, // Security: don't follow symlinks
            max_depth: 100,
            include_hidden: false,
            respect_gitignore: true,
        }
    }
}

/// Platform-agnostic filesystem abstraction.
///
/// This trait provides async filesystem operations that work on both
/// native platforms (using std::fs) and WASM (using in-memory storage).
///
/// # Design Decisions
///
/// ## Async vs Sync
///
/// All methods are async to support both:
/// - **Native**: I/O operations offloaded to blocking thread pool via tokio::spawn_blocking
/// - **WASM**: In-memory operations that complete immediately
///
/// ## Error Handling
///
/// Uses `std::io::Result<T>` for compatibility:
/// - Native: Direct mapping from std::fs errors
/// - WASM: Construct io::Error with appropriate ErrorKind
#[async_trait::async_trait]
pub trait FileSystem: Send + Sync {
    /// Check if a path exists.
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Read file contents as a string.
    ///
    /// # Errors
    ///
    /// Returns `io::ErrorKind::NotFound` if file doesn't exist.
    /// Returns `io::ErrorKind::InvalidData` if file is not valid UTF-8.
    async fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Read file contents as bytes.
    ///
    /// Use this when you need binary data or want to avoid UTF-8 validation overhead.
    async fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Get file/directory metadata.
    ///
    /// Returns metadata even if the file doesn't exist (exists=false).
    /// This avoids the need for separate exists() + metadata() calls.
    async fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// Write string contents to a file.
    ///
    /// # Security
    ///
    /// - Path is validated against project root
    /// - Parent directories are NOT created automatically
    /// - Overwrites existing files
    async fn write(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Write bytes to a file.
    async fn write_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Remove a file.
    ///
    /// # Security
    ///
    /// - Only removes files, not directories
    /// - Path must be within project root
    /// - Fails if file is a symlink (unless explicitly allowed)
    async fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Atomically rename a file.
    ///
    /// Used for atomic file updates (write to .tmp, then rename).
    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Create a directory.
    async fn create_dir(&self, path: &Path) -> io::Result<()>;

    /// Create a directory and all parent directories.
    async fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Discover files matching extensions and patterns.
    ///
    /// This is the primary file discovery operation that replaces WalkDir.
    ///
    /// # Parameters
    ///
    /// - `root`: Starting directory (must be within project root)
    /// - `extensions`: File extensions to include (e.g., [".ts", ".js"])
    /// - `ignore_patterns`: Glob patterns to ignore (e.g., ["node_modules/**", "*.test.ts"])
    /// - `options`: Additional discovery options
    ///
    /// # Returns
    ///
    /// Set of absolute paths to discovered files.
    async fn discover_files(
        &self,
        root: &Path,
        extensions: &[&str],
        ignore_patterns: &[&str],
        options: &DiscoveryOptions,
    ) -> io::Result<HashSet<PathBuf>>;

    /// Normalize a path (replaces canonicalize for WASM compatibility).
    ///
    /// - **Native**: Uses `fs::canonicalize()` for real filesystem paths
    /// - **WASM**: Performs syntactic normalization (removes `.`, `..`, etc.)
    ///
    /// # Security
    ///
    /// Returns an error if normalized path escapes project root.
    async fn normalize_path(&self, path: &Path) -> io::Result<PathBuf>;

    /// Get the project root this filesystem is scoped to.
    ///
    /// All operations are validated against this root for security.
    fn project_root(&self) -> &Path;
}

