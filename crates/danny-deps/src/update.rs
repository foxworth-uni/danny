//! Safe file update operations

use crate::Result;
use danny_fs::FileSystem;
use std::path::Path;
use std::sync::Arc;

/// File updater that performs atomic writes
pub struct FileUpdater {
    dry_run: bool,
}

impl FileUpdater {
    /// Create a new file updater
    ///
    /// # Arguments
    /// * `dry_run` - If true, don't actually write changes (just validate)
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Atomically update a file using FileSystem
    ///
    /// Strategy:
    /// 1. Write to temporary file in same directory (ensures same filesystem)
    /// 2. Verify contents can be read back
    /// 3. Rename (atomic on POSIX, best-effort on Windows)
    ///
    /// # Arguments
    /// * `fs` - FileSystem instance for file operations
    /// * `path` - Path to the file to update
    /// * `new_contents` - New file contents
    ///
    /// # Errors
    /// Returns an error if the file cannot be written or renamed
    pub async fn update_file<F: FileSystem>(
        &self,
        fs: &Arc<F>,
        path: &Path,
        new_contents: String,
    ) -> Result<()> {
        if self.dry_run {
            // In dry-run mode, just validate that the contents are valid UTF-8
            // (which they already are since we have a String)
            return Ok(());
        }

        // Create temp file in same directory (ensures same filesystem)
        // Generate temp filename: original.ext.tmp
        let temp_path = path.with_extension(format!(
            "{}.tmp",
            path.extension().and_then(|ext| ext.to_str()).unwrap_or("")
        ));

        // Write to temp file
        fs.write(&temp_path, &new_contents).await?;

        // Verify we can read it back (basic sanity check)
        let _ = fs.read_to_string(&temp_path).await?;

        // Atomic rename (atomic on POSIX, best-effort on Windows)
        fs.rename(&temp_path, path).await?;

        Ok(())
    }

    /// Update file synchronously (for non-async contexts)
    ///
    /// This creates a temporary runtime if needed.
    pub fn update_file_sync(&self, path: &Path, new_contents: String) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }

        // Create temp file in same directory
        let temp_path = path.with_extension(format!(
            "{}.tmp",
            path.extension().and_then(|ext| ext.to_str()).unwrap_or("")
        ));

        // Write to temp file synchronously
        std::fs::write(&temp_path, &new_contents)?;

        // Verify we can read it back
        let _ = std::fs::read_to_string(&temp_path)?;

        // Atomic rename
        std::fs::rename(&temp_path, path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use danny_fs::NativeFileSystem;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_update_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());

        // Create initial file
        std::fs::write(&file_path, "old content").unwrap();

        // Update it
        let updater = FileUpdater::new(false);
        updater
            .update_file(&fs, &file_path, "new content".to_string())
            .await
            .unwrap();

        // Verify content
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let fs = Arc::new(NativeFileSystem::new(temp_dir.path()).unwrap());

        // Create initial file
        std::fs::write(&file_path, "old content").unwrap();

        // Try to update in dry-run mode
        let updater = FileUpdater::new(true);
        updater
            .update_file(&fs, &file_path, "new content".to_string())
            .await
            .unwrap();

        // Verify content unchanged
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "old content");
    }

    #[test]
    fn test_update_file_sync() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        std::fs::write(&file_path, "old content").unwrap();

        // Update it synchronously
        let updater = FileUpdater::new(false);
        updater
            .update_file_sync(&file_path, "new content".to_string())
            .unwrap();

        // Verify content
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }
}
