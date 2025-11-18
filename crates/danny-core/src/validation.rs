//! Input validation and security checks.

use crate::error::{Error, Result};
use std::path::Path;

/// Validates that path is within project root (prevents path traversal)
pub fn validate_path(path: &Path, project_root: &Path) -> Result<()> {
    let canonical_path = path.canonicalize().map_err(|e| Error::InvalidPath {
        path: path.to_path_buf(),
        reason: format!("Cannot canonicalize: {}", e),
    })?;

    let canonical_root = project_root.canonicalize().map_err(|e| {
        Error::InvalidPath {
            path: project_root.to_path_buf(),
            reason: format!("Cannot canonicalize project root: {}", e),
        }
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::PathTraversal {
            attempted_path: path.to_path_buf(),
            project_root: project_root.to_path_buf(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_valid_path() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let valid_path = project_root.join("src/file.ts");

        fs::create_dir_all(valid_path.parent().unwrap()).unwrap();
        fs::write(&valid_path, "").unwrap();

        let result = validate_path(&valid_path, project_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        fs::create_dir_all(&project_root).unwrap();

        // Create a file outside project root
        let outside_file = temp.path().join("outside.txt");
        fs::write(&outside_file, "").unwrap();

        let result = validate_path(&outside_file, &project_root);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::PathTraversal { .. }));
    }
}

