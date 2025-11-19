//! Core types for dependency management

use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a dependency ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Ecosystem {
    /// Rust (Cargo)
    Rust,
    /// JavaScript/TypeScript (npm, pnpm, yarn)
    JavaScript,
    // Future: Python, Go, Ruby, etc.
    // This is non_exhaustive to allow adding new ecosystems without breaking API
}

/// Type of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyType {
    /// Runtime dependency
    Runtime,
    /// Development dependency
    Dev,
    /// Build dependency (Cargo only)
    Build,
    /// Peer dependency (npm only)
    Peer,
    /// Optional dependency
    Optional,
}

/// Version requirement specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    /// The raw version requirement string (e.g., "^1.0.0", ">=2.0, <3.0")
    pub raw: String,
    /// The ecosystem this version belongs to (affects parsing)
    pub ecosystem: Ecosystem,
}

/// Represents a single dependency
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version requirement
    pub version_req: VersionReq,
    /// Type of dependency
    pub dep_type: DependencyType,
    /// Optional features (Cargo) or extras (Python)
    pub features: Vec<String>,
    /// Whether this is a workspace dependency
    pub workspace: bool,
    /// Optional source (git, path, registry)
    pub source: Option<String>,
}

/// Represents a parsed dependency file (Cargo.toml, package.json, etc.)
#[derive(Debug, Clone)]
pub struct DependencyFile {
    /// Path to the file
    pub path: PathBuf,
    /// Ecosystem
    pub ecosystem: Ecosystem,
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// All dependencies grouped by type
    pub dependencies: HashMap<DependencyType, Vec<Dependency>>,
    /// Whether this is a workspace root
    pub is_workspace_root: bool,
    /// Workspace members (if this is a workspace root)
    pub workspace_members: Vec<PathBuf>,
}

impl DependencyFile {
    /// Get all dependencies across all types
    pub fn all_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies.values().flatten()
    }

    /// Get dependencies of a specific type
    pub fn dependencies_of_type(&self, dep_type: DependencyType) -> &[Dependency] {
        self.dependencies
            .get(&dep_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Find a dependency by name
    pub fn find_dependency(&self, name: &str) -> Option<&Dependency> {
        self.all_dependencies().find(|d| d.name == name)
    }
}

/// Represents an update to be applied
#[derive(Debug, Clone)]
pub struct DependencyUpdate {
    /// Package name
    pub package: String,
    /// New version requirement
    pub new_version: String,
    /// Dependency type to update
    pub dep_type: DependencyType,
}

/// Result of applying updates
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// Path to the updated file
    pub file: PathBuf,
    /// List of updates applied
    pub updates: Vec<AppliedUpdate>,
    /// Whether this was a dry run
    pub dry_run: bool,
}

/// An individual update that was applied
#[derive(Debug, Clone)]
pub struct AppliedUpdate {
    /// Package name
    pub package: String,
    /// Old version requirement
    pub old_version: String,
    /// New version requirement
    pub new_version: String,
    /// Dependency type
    pub dep_type: DependencyType,
}
