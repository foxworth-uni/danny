use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Security-related errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Path does not exist: {0}")]
    PathDoesNotExist(PathBuf),

    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Path is not readable: {0}")]
    NotReadable(PathBuf),

    #[error("Invalid project ID: {0}. Must contain only alphanumeric characters and hyphens")]
    InvalidProjectId(String),

    #[error("Project name too long: {0} characters (max 100)")]
    NameTooLong(usize),

    #[error("Project name contains invalid characters")]
    InvalidName,
}

/// Validate and canonicalize a project path
///
/// This function ensures:
/// 1. The path exists
/// 2. It's a directory
/// 3. It's readable
/// 4. Symlinks are resolved (canonicalized)
///
/// # Security
/// Canonicalization prevents path traversal attacks and ensures we're
/// working with the actual filesystem location.
pub fn validate_project_path(path: &Path) -> Result<PathBuf, SecurityError> {
    // Canonicalize to resolve symlinks and normalize the path
    let canonical = path
        .canonicalize()
        .map_err(|_| SecurityError::PathDoesNotExist(path.to_path_buf()))?;

    // Ensure it's a directory
    if !canonical.is_dir() {
        return Err(SecurityError::NotADirectory(canonical));
    }

    // Ensure we can read it
    if fs::read_dir(&canonical).is_err() {
        return Err(SecurityError::NotReadable(canonical));
    }

    Ok(canonical)
}

/// Validate a project ID
///
/// Project IDs must:
/// - Be 1-50 characters long
/// - Contain only alphanumeric characters and hyphens
/// - Not start or end with a hyphen
pub fn validate_project_id(id: &str) -> Result<(), SecurityError> {
    if id.is_empty() || id.len() > 50 {
        return Err(SecurityError::InvalidProjectId(id.to_string()));
    }

    if id.starts_with('-') || id.ends_with('-') {
        return Err(SecurityError::InvalidProjectId(id.to_string()));
    }

    if !id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(SecurityError::InvalidProjectId(id.to_string()));
    }

    Ok(())
}

/// Validate a project name
///
/// Project names must:
/// - Be 1-100 characters long
/// - Not contain control characters
pub fn validate_project_name(name: &str) -> Result<(), SecurityError> {
    if name.is_empty() {
        return Err(SecurityError::InvalidName);
    }

    if name.len() > 100 {
        return Err(SecurityError::NameTooLong(name.len()));
    }

    if name.chars().any(|c| c.is_control()) {
        return Err(SecurityError::InvalidName);
    }

    Ok(())
}

/// Set restrictive permissions on config file (Unix only)
#[cfg(unix)]
pub fn set_config_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600); // rw------- (user read/write only)
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// Set config permissions (no-op on Windows for now)
#[cfg(not(unix))]
pub fn set_config_permissions(_path: &Path) -> std::io::Result<()> {
    // Windows: Could use ACLs but that's more complex
    // For now, we rely on the default permissions
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_existing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = validate_project_path(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path/12345");
        let result = validate_project_path(&path);
        assert!(matches!(result, Err(SecurityError::PathDoesNotExist(_))));
    }

    #[test]
    fn test_validate_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let result = validate_project_path(&file_path);
        assert!(matches!(result, Err(SecurityError::NotADirectory(_))));
    }

    #[test]
    fn test_valid_project_ids() {
        assert!(validate_project_id("my-project").is_ok());
        assert!(validate_project_id("project123").is_ok());
        assert!(validate_project_id("test-project-1").is_ok());
        assert!(validate_project_id("a").is_ok());
    }

    #[test]
    fn test_invalid_project_ids() {
        assert!(validate_project_id("").is_err());
        assert!(validate_project_id("-leading").is_err());
        assert!(validate_project_id("trailing-").is_err());
        assert!(validate_project_id("has spaces").is_err());
        assert!(validate_project_id("has/slash").is_err());
        assert!(validate_project_id("has@special").is_err());
        assert!(validate_project_id(&"a".repeat(51)).is_err());
    }

    #[test]
    fn test_valid_project_names() {
        assert!(validate_project_name("My Project").is_ok());
        assert!(validate_project_name("Project (2025)").is_ok());
        assert!(validate_project_name("Test-Project_123").is_ok());
    }

    #[test]
    fn test_invalid_project_names() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name(&"a".repeat(101)).is_err());
        assert!(validate_project_name("Has\nNewline").is_err());
        assert!(validate_project_name("Has\tTab").is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_set_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "test").unwrap();

        set_config_permissions(&config_path).unwrap();

        let metadata = fs::metadata(&config_path).unwrap();
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}
