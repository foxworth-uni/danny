use anyhow::Result;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Find nearest package.json by walking up directory tree
pub fn find_nearest_package_json(start: &Path) -> Result<Option<PathBuf>> {
    let mut current = if start.is_file() {
        start.parent()
    } else {
        Some(start)
    };

    while let Some(dir) = current {
        let package_json = dir.join("package.json");

        // Security: Use File::open to avoid TOCTOU race condition
        if File::open(&package_json).is_ok() {
            return Ok(Some(package_json));
        }

        current = dir.parent();
    }

    Ok(None)
}
