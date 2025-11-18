use anyhow::{Context, Result};
use danny_config::{AnalysisTarget, PackageTarget, FilesTarget};
use danny_core::Error;
use std::path::{Path, PathBuf};

pub struct EntryPointDetector {
    working_dir: PathBuf,
}

impl EntryPointDetector {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// Detect analysis target from CLI paths
    pub fn detect_target(
        &self,
        paths: &[PathBuf],
        force_package_mode: bool,
    ) -> Result<AnalysisTarget> {
        // Empty paths defaults to current directory (package mode)
        if paths.is_empty() {
            return self.detect_package_from_directory(&self.working_dir);
        }

        // Single path detection
        if paths.len() == 1 {
            let path = &paths[0];

            if force_package_mode {
                return self.find_package_for_path(path);
            }

            // Check if it's a directory
            if path.is_dir() {
                return self.detect_package_from_directory(path);
            }

            // Single file -> files mode
            return self.create_files_target(vec![path.clone()]);
        }

        // Multiple paths -> files mode (unless forced)
        if force_package_mode {
            self.find_package_for_paths(paths)
        } else {
            self.create_files_target(paths.to_vec())
        }
    }

    fn detect_package_from_directory(&self, dir: &Path) -> Result<AnalysisTarget> {
        // Look for package.json
        let package_json = dir.join("package.json");

        // TOCTOU fix: Try to open file instead of checking existence
        use std::fs::File;
        match File::open(&package_json) {
            Ok(_) => {
                // File exists, proceed
                self.create_package_target(dir, package_json)
            }
            Err(_) => {
                // No package.json found - error
                Err(Error::NoPackageJson {
                    searched: dir.to_path_buf(),
                }
                .into())
            }
        }
    }

    fn create_package_target(&self, root: &Path, package_json: PathBuf) -> Result<AnalysisTarget> {
        // Parse package.json for entry points
        let entry_points = super::package::extract_entry_points(&package_json)?;

        // Detect framework
        let framework = super::package::detect_framework(root)?;

        // Check for node_modules
        let has_node_modules = root.join("node_modules").is_dir();

        Ok(AnalysisTarget::Package(PackageTarget {
            root: root.to_path_buf(),
            entry_points,
            framework,
            has_node_modules,
        }))
    }

    fn create_files_target(&self, files: Vec<PathBuf>) -> Result<AnalysisTarget> {
        // Security: Validate file count and paths (includes path traversal check)
        super::security::validate_files_for_analysis(&files, &self.working_dir)
            .context("File validation failed")?;

        // Find nearby package.json for suggestions
        let nearby_package = if let Some(first_file) = files.first() {
            super::files::find_nearest_package_json(first_file)?
        } else {
            None
        };

        Ok(AnalysisTarget::Files(FilesTarget {
            files,
            working_dir: self.working_dir.clone(),
            nearby_package,
        }))
    }

    fn find_package_for_path(&self, path: &Path) -> Result<AnalysisTarget> {
        // Security: Validate path is within working directory
        super::security::validate_path_within_root(path, &self.working_dir)
            .context("Path is outside project root")?;

        // Find the package.json that contains this path
        if let Some(package_json) = super::files::find_nearest_package_json(path)? {
            let root = package_json
                .parent()
                .ok_or_else(|| Error::InvalidPath {
                    path: package_json.clone(),
                    reason: "package.json has no parent directory".to_string(),
                })?;
            self.create_package_target(root, package_json.clone())
        } else {
            Err(Error::NoPackageJson {
                searched: path.to_path_buf(),
            }
            .into())
        }
    }

    fn find_package_for_paths(&self, paths: &[PathBuf]) -> Result<AnalysisTarget> {
        // Find common package root
        if let Some(first) = paths.first() {
            self.find_package_for_path(first)
        } else {
            anyhow::bail!("No paths provided for analysis")
        }
    }
}

