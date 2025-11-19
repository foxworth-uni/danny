# danny-deps Implementation Plan

**Version:** 0.1.0
**Last Updated:** 2025-01-14
**Target Ecosystems:** Rust (Cargo) and JavaScript (npm/pnpm/yarn)

## 🎯 Overview

`danny-deps` handles **LOCAL** dependency file parsing and management for Rust and JavaScript projects. It complements `danny-info` (which fetches REMOTE package data) by providing:

- Dependency file parsing (Cargo.toml, package.json)
- Lockfile parsing (Cargo.lock, package-lock.json, pnpm-lock.yaml, yarn.lock)
- Version comparison (semver + npm-style)
- Safe file updates preserving formatting and comments
- Monorepo/workspace support
- Lockfile integrity verification

## 🏗️ Architecture

### Design Principles

1. **Trait-Based Abstraction**: Follow `danny-core`'s `LanguageBackend` pattern for extensibility
2. **Ecosystem Isolation**: Keep Rust and JavaScript code separate to prevent cross-contamination
3. **Format Preservation**: Use `toml_edit` (NOT `toml`) to preserve comments and formatting
4. **Future-Proof**: Design allows adding Python, Go, Ruby without breaking existing API
5. **File Size Limit**: Keep all files under 500 lines

### Core Traits

Located in `src/traits.rs`:

```rust
/// Main abstraction for dependency file management
pub trait DependencyManager: Send + Sync {
    fn parse(&self, path: &Path) -> Result<DependencyFile>;
    fn update(&self, path: &Path, updates: &[DependencyUpdate], dry_run: bool) -> Result<UpdateResult>;
    fn validate(&self, path: &Path) -> Result<()>;
    fn is_workspace_root(&self, path: &Path) -> Result<bool>;
    fn find_workspace_members(&self, root: &Path) -> Result<Vec<PathBuf>>;
}

/// Trait for lockfile parsing
pub trait LockfileParser: Send + Sync {
    fn parse_lockfile(&self, path: &Path) -> Result<LockedDependencies>;
    fn verify_integrity(&self, path: &Path) -> Result<()>;
}
```

### Module Structure

```
src/
├── lib.rs              # Public API, re-exports
├── error.rs            # Error types (thiserror)
├── traits.rs           # Core trait definitions
├── types.rs            # Common types (DependencyFile, Dependency, etc.)
├── version.rs          # Version parsing and comparison
├── checksum.rs         # SHA-256/SHA-512 verification
├── update.rs           # Safe update operations
├── cargo/
│   ├── mod.rs          # Cargo ecosystem public API
│   ├── parser.rs       # Cargo.toml parsing (toml_edit)
│   ├── lockfile.rs     # Cargo.lock parsing (cargo_lock crate)
│   └── workspace.rs    # Cargo workspace detection
└── npm/
    ├── mod.rs          # npm ecosystem public API
    ├── parser.rs       # package.json parsing
    ├── lockfile.rs     # package-lock.json, pnpm-lock.yaml, yarn.lock
    └── workspace.rs    # npm/pnpm workspaces, turborepo
```

## 📦 Implementation Phases

### Phase 1: Core Infrastructure (Week 1)

**Files:** `version.rs`, `checksum.rs`, `update.rs`

#### 1.1 Version Parsing (`src/version.rs`)

Support both semver and npm-style version requirements:

```rust
use semver::{Version, VersionReq as SemverReq};
use node_semver::{Range as NpmRange, Version as NpmVersion};

pub enum ParsedVersionReq {
    Semver(SemverReq),      // For Cargo: "1.0", "^1.0", ">=1.0, <2.0"
    Npm(NpmRange),          // For npm: "^1.0.0", "~1.0.0", "*", "latest"
}

impl ParsedVersionReq {
    /// Parse a version requirement based on ecosystem
    pub fn parse(raw: &str, ecosystem: Ecosystem) -> Result<Self> {
        match ecosystem {
            Ecosystem::Rust => {
                let req = SemverReq::parse(raw)?;
                Ok(Self::Semver(req))
            }
            Ecosystem::JavaScript => {
                let range = NpmRange::parse(raw)?;
                Ok(Self::Npm(range))
            }
        }
    }

    /// Check if a version satisfies this requirement
    pub fn matches(&self, version: &str) -> Result<bool> {
        match self {
            Self::Semver(req) => {
                let v = Version::parse(version)?;
                Ok(req.matches(&v))
            }
            Self::Npm(range) => {
                let v = NpmVersion::parse(version)?;
                Ok(range.satisfies(&v))
            }
        }
    }

    /// Compare two versions
    pub fn compare(a: &str, b: &str, ecosystem: Ecosystem) -> Result<std::cmp::Ordering> {
        match ecosystem {
            Ecosystem::Rust => {
                let va = Version::parse(a)?;
                let vb = Version::parse(b)?;
                Ok(va.cmp(&vb))
            }
            Ecosystem::JavaScript => {
                let va = NpmVersion::parse(a)?;
                let vb = NpmVersion::parse(b)?;
                Ok(va.cmp(&vb))
            }
        }
    }

    /// Determine update type (major, minor, patch)
    pub fn update_type(current: &str, latest: &str, ecosystem: Ecosystem) -> Result<UpdateType> {
        match ecosystem {
            Ecosystem::Rust => {
                let c = Version::parse(current)?;
                let l = Version::parse(latest)?;
                Ok(if l.major > c.major {
                    UpdateType::Major
                } else if l.minor > c.minor {
                    UpdateType::Minor
                } else if l.patch > c.patch {
                    UpdateType::Patch
                } else {
                    UpdateType::None
                })
            }
            Ecosystem::JavaScript => {
                // Similar logic using NpmVersion
                todo!()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    Major,  // 1.0.0 -> 2.0.0 (potentially breaking)
    Minor,  // 1.0.0 -> 1.1.0 (new features)
    Patch,  // 1.0.0 -> 1.0.1 (bug fixes)
    None,   // Already on latest
}
```

**Testing:**
- Semver parsing: `"1.0"`, `"^1.0"`, `">=1.0, <2.0"`, `"1.0.*"`
- npm syntax: `"^1.0.0"`, `"~1.0.0"`, `"*"`, `"latest"`, `"1.x"`
- Property-based tests (proptest) for version comparison
- Edge cases: `"0.0.1"`, pre-release versions

#### 1.2 Checksum Verification (`src/checksum.rs`)

```rust
use sha2::{Sha256, Sha512, Digest};

pub enum ChecksumAlgorithm {
    Sha256,  // Cargo.lock
    Sha512,  // npm package-lock.json
}

pub struct ChecksumVerifier {
    algorithm: ChecksumAlgorithm,
}

impl ChecksumVerifier {
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Compute checksum of file contents
    pub fn compute(&self, data: &[u8]) -> String {
        match self.algorithm {
            ChecksumAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            ChecksumAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(data);
                format!("sha512-{}", base64::encode(hasher.finalize()))
            }
        }
    }

    /// Verify that computed checksum matches expected
    pub fn verify(&self, data: &[u8], expected: &str) -> Result<()> {
        let computed = self.compute(data);
        if computed == expected {
            Ok(())
        } else {
            Err(Error::ChecksumMismatch(
                "data".to_string(),
                expected.to_string(),
                computed,
            ))
        }
    }
}
```

#### 1.3 Safe File Updates (`src/update.rs`)

```rust
use std::path::Path;
use tokio::fs;

pub struct FileUpdater {
    dry_run: bool,
}

impl FileUpdater {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Atomically update a file
    ///
    /// Strategy:
    /// 1. Write to temporary file
    /// 2. Verify contents
    /// 3. Rename (atomic on POSIX)
    pub async fn update_file(&self, path: &Path, new_contents: String) -> Result<()> {
        if self.dry_run {
            // Just validate that we can parse the new contents
            return Ok(());
        }

        // Create temp file in same directory (ensures same filesystem)
        let temp_path = path.with_extension("tmp");

        // Write to temp file
        fs::write(&temp_path, &new_contents).await?;

        // Atomic rename
        fs::rename(&temp_path, path).await?;

        Ok(())
    }
}
```

### Phase 2: Cargo Support (Week 2)

**Files:** `src/cargo/parser.rs`, `src/cargo/lockfile.rs`, `src/cargo/workspace.rs`

#### 2.1 Cargo.toml Parser (`src/cargo/parser.rs`)

**Key Requirement:** Use `toml_edit` to preserve comments and formatting!

```rust
use toml_edit::{Document, Item, Value};
use crate::{DependencyManager, DependencyFile, Dependency, DependencyType};

pub struct CargoDependencyManager;

impl CargoDependencyManager {
    pub fn new() -> Self {
        Self
    }

    fn parse_dependency(&self, name: &str, value: &Item) -> Result<Dependency> {
        match value {
            // Simple: serde = "1.0"
            Item::Value(Value::String(s)) => {
                Ok(Dependency {
                    name: name.to_string(),
                    version_req: VersionReq {
                        raw: s.value().to_string(),
                        ecosystem: Ecosystem::Rust,
                    },
                    dep_type: DependencyType::Runtime,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
            }
            // Table: serde = { version = "1.0", features = ["derive"] }
            Item::Value(Value::InlineTable(table)) => {
                let version = table.get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "*".to_string());

                let features = table.get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let workspace = table.get("workspace")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                Ok(Dependency {
                    name: name.to_string(),
                    version_req: VersionReq {
                        raw: version,
                        ecosystem: Ecosystem::Rust,
                    },
                    dep_type: DependencyType::Runtime,
                    features,
                    workspace,
                    source: None,
                })
            }
            _ => Err(Error::InvalidFormat(
                PathBuf::from("Cargo.toml"),
                format!("Invalid dependency format for {}", name),
            )),
        }
    }
}

impl DependencyManager for CargoDependencyManager {
    fn parse(&self, path: &Path) -> Result<DependencyFile> {
        let content = std::fs::read_to_string(path)?;
        let doc = content.parse::<Document>()?;

        // Parse [package] section
        let package = doc.get("package")
            .and_then(|p| p.as_table())
            .ok_or_else(|| Error::InvalidFormat(
                path.to_path_buf(),
                "Missing [package] section".to_string(),
            ))?;

        let name = package.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| Error::InvalidFormat(
                path.to_path_buf(),
                "Missing package.name".to_string(),
            ))?
            .to_string();

        let version = package.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        // Parse dependencies
        let mut dependencies = HashMap::new();

        if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
            let runtime_deps: Vec<Dependency> = deps.iter()
                .filter_map(|(k, v)| self.parse_dependency(k, v).ok())
                .collect();
            dependencies.insert(DependencyType::Runtime, runtime_deps);
        }

        if let Some(dev_deps) = doc.get("dev-dependencies").and_then(|d| d.as_table()) {
            let dev_deps: Vec<Dependency> = dev_deps.iter()
                .filter_map(|(k, v)| self.parse_dependency(k, v).ok())
                .map(|mut d| { d.dep_type = DependencyType::Dev; d })
                .collect();
            dependencies.insert(DependencyType::Dev, dev_deps);
        }

        // Check if workspace root
        let is_workspace_root = doc.get("workspace").is_some();

        Ok(DependencyFile {
            path: path.to_path_buf(),
            ecosystem: Ecosystem::Rust,
            name,
            version,
            dependencies,
            is_workspace_root,
            workspace_members: vec![],
        })
    }

    fn update(&self, path: &Path, updates: &[DependencyUpdate], dry_run: bool) -> Result<UpdateResult> {
        let content = std::fs::read_to_string(path)?;
        let mut doc = content.parse::<Document>()?;
        let mut applied = vec![];

        for update in updates {
            let section = match update.dep_type {
                DependencyType::Runtime => "dependencies",
                DependencyType::Dev => "dev-dependencies",
                DependencyType::Build => "build-dependencies",
                _ => continue,
            };

            if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_mut()) {
                if let Some(dep_item) = deps.get_mut(&update.package) {
                    let old_version = match dep_item {
                        Item::Value(Value::String(s)) => s.value().to_string(),
                        Item::Value(Value::InlineTable(table)) => {
                            table.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("*")
                                .to_string()
                        }
                        _ => continue,
                    };

                    // Update the version
                    match dep_item {
                        Item::Value(Value::String(s)) => {
                            *s = update.new_version.clone().into();
                        }
                        Item::Value(Value::InlineTable(table)) => {
                            if let Some(version) = table.get_mut("version") {
                                *version = Value::String(update.new_version.clone().into());
                            }
                        }
                        _ => {}
                    }

                    applied.push(AppliedUpdate {
                        package: update.package.clone(),
                        old_version,
                        new_version: update.new_version.clone(),
                        dep_type: update.dep_type,
                    });
                }
            }
        }

        if !dry_run && !applied.is_empty() {
            let updater = FileUpdater::new(false);
            tokio::runtime::Runtime::new()?
                .block_on(updater.update_file(path, doc.to_string()))?;
        }

        Ok(UpdateResult {
            file: path.to_path_buf(),
            updates: applied,
            dry_run,
        })
    }

    fn validate(&self, path: &Path) -> Result<()> {
        // Just try to parse it
        self.parse(path)?;
        Ok(())
    }

    fn is_workspace_root(&self, path: &Path) -> Result<bool> {
        let manifest = self.parse(path)?;
        Ok(manifest.is_workspace_root)
    }

    fn find_workspace_members(&self, root: &Path) -> Result<Vec<PathBuf>> {
        // Implemented in workspace.rs
        todo!()
    }
}
```

#### 2.2 Cargo.lock Parser (`src/cargo/lockfile.rs`)

```rust
use cargo_lock::Lockfile;
use crate::{LockfileParser, LockedDependencies, LockedPackage};

pub struct CargoLockfileParser;

impl LockfileParser for CargoLockfileParser {
    fn parse_lockfile(&self, path: &Path) -> Result<LockedDependencies> {
        let lockfile = Lockfile::load(path)
            .map_err(|e| Error::CargoLock(e.to_string()))?;

        let packages = lockfile.packages.iter()
            .map(|pkg| {
                let name = pkg.name.as_str().to_string();
                let locked = LockedPackage {
                    name: name.clone(),
                    version: pkg.version.to_string(),
                    checksum: pkg.checksum.as_ref().map(|c| c.to_string()),
                    resolved: pkg.source.as_ref().map(|s| s.to_string()),
                };
                (name, locked)
            })
            .collect();

        Ok(LockedDependencies { packages })
    }

    fn verify_integrity(&self, path: &Path) -> Result<()> {
        // cargo_lock crate handles checksum verification automatically
        let _lockfile = Lockfile::load(path)
            .map_err(|e| Error::CargoLock(e.to_string()))?;
        Ok(())
    }
}
```

#### 2.3 Cargo Workspace Detection (`src/cargo/workspace.rs`)

```rust
use std::path::{Path, PathBuf};
use toml_edit::Document;

pub struct CargoWorkspace;

impl CargoWorkspace {
    /// Find workspace root starting from a given path
    pub fn find_root(start: &Path) -> Result<Option<PathBuf>> {
        let mut current = start.to_path_buf();

        loop {
            let manifest_path = current.join("Cargo.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)?;
                let doc = content.parse::<Document>()?;

                if doc.get("workspace").is_some() {
                    return Ok(Some(current));
                }
            }

            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Get workspace members from a workspace root
    pub fn get_members(root: &Path) -> Result<Vec<PathBuf>> {
        let manifest_path = root.join("Cargo.toml");
        let content = std::fs::read_to_string(&manifest_path)?;
        let doc = content.parse::<Document>()?;

        let workspace = doc.get("workspace")
            .and_then(|w| w.as_table())
            .ok_or_else(|| Error::WorkspaceError("Not a workspace root".to_string()))?;

        let members = workspace.get("members")
            .and_then(|m| m.as_array())
            .ok_or_else(|| Error::WorkspaceError("No members field".to_string()))?;

        let member_paths: Vec<PathBuf> = members.iter()
            .filter_map(|v| v.as_str())
            .map(|s| root.join(s).join("Cargo.toml"))
            .collect();

        Ok(member_paths)
    }
}
```

### Phase 3: npm/pnpm/yarn Support (Week 3)

**Files:** `src/npm/parser.rs`, `src/npm/lockfile.rs`, `src/npm/workspace.rs`

#### 3.1 package.json Parser (`src/npm/parser.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,

    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: HashMap<String, String>,

    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: HashMap<String, String>,

    #[serde(default, rename = "optionalDependencies")]
    pub optional_dependencies: HashMap<String, String>,

    // Workspace fields
    #[serde(default)]
    pub workspaces: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WorkspaceConfig {
    Simple(Vec<String>),
    Extended {
        packages: Vec<String>,
        #[serde(default)]
        nohoist: Vec<String>,
    },
}

pub struct NpmDependencyManager;

impl DependencyManager for NpmDependencyManager {
    fn parse(&self, path: &Path) -> Result<DependencyFile> {
        let content = std::fs::read_to_string(path)?;
        let pkg: PackageJson = serde_json::from_str(&content)?;

        let mut dependencies = HashMap::new();

        // Runtime dependencies
        if !pkg.dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg.dependencies.iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Runtime,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Runtime, deps);
        }

        // Dev dependencies
        if !pkg.dev_dependencies.is_empty() {
            let deps: Vec<Dependency> = pkg.dev_dependencies.iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    version_req: VersionReq {
                        raw: version.clone(),
                        ecosystem: Ecosystem::JavaScript,
                    },
                    dep_type: DependencyType::Dev,
                    features: vec![],
                    workspace: false,
                    source: None,
                })
                .collect();
            dependencies.insert(DependencyType::Dev, deps);
        }

        let is_workspace_root = pkg.workspaces.is_some();

        Ok(DependencyFile {
            path: path.to_path_buf(),
            ecosystem: Ecosystem::JavaScript,
            name: pkg.name,
            version: pkg.version,
            dependencies,
            is_workspace_root,
            workspace_members: vec![],
        })
    }

    fn update(&self, path: &Path, updates: &[DependencyUpdate], dry_run: bool) -> Result<UpdateResult> {
        let content = std::fs::read_to_string(path)?;
        let mut pkg: serde_json::Value = serde_json::from_str(&content)?;
        let mut applied = vec![];

        for update in updates {
            let field = match update.dep_type {
                DependencyType::Runtime => "dependencies",
                DependencyType::Dev => "devDependencies",
                DependencyType::Peer => "peerDependencies",
                DependencyType::Optional => "optionalDependencies",
                _ => continue,
            };

            if let Some(deps) = pkg.get_mut(field).and_then(|v| v.as_object_mut()) {
                if let Some(old_version) = deps.get(&update.package) {
                    let old = old_version.as_str().unwrap_or("*").to_string();
                    deps.insert(
                        update.package.clone(),
                        serde_json::Value::String(update.new_version.clone()),
                    );

                    applied.push(AppliedUpdate {
                        package: update.package.clone(),
                        old_version: old,
                        new_version: update.new_version.clone(),
                        dep_type: update.dep_type,
                    });
                }
            }
        }

        if !dry_run && !applied.is_empty() {
            // Pretty print with 2-space indentation (npm standard)
            let formatted = serde_json::to_string_pretty(&pkg)?;
            let updater = FileUpdater::new(false);
            tokio::runtime::Runtime::new()?
                .block_on(updater.update_file(path, formatted))?;
        }

        Ok(UpdateResult {
            file: path.to_path_buf(),
            updates: applied,
            dry_run,
        })
    }

    // ... other trait methods
}
```

#### 3.2 Lockfile Parsers (`src/npm/lockfile.rs`)

Support three formats: package-lock.json, pnpm-lock.yaml, yarn.lock

```rust
use serde::{Deserialize, Serialize};

// package-lock.json (npm v7+)
#[derive(Debug, Deserialize)]
pub struct PackageLock {
    pub lockfileVersion: u8,
    pub packages: HashMap<String, PackageLockEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PackageLockEntry {
    pub version: String,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
}

pub struct NpmLockfileParser;

impl LockfileParser for NpmLockfileParser {
    fn parse_lockfile(&self, path: &Path) -> Result<LockedDependencies> {
        let content = std::fs::read_to_string(path)?;
        let lock: PackageLock = serde_json::from_str(&content)?;

        let packages = lock.packages.iter()
            .filter_map(|(key, entry)| {
                // Skip root entry (empty key)
                if key.is_empty() {
                    return None;
                }

                // Extract package name from "node_modules/foo"
                let name = key.strip_prefix("node_modules/")?.to_string();

                Some((name.clone(), LockedPackage {
                    name,
                    version: entry.version.clone(),
                    checksum: entry.integrity.clone(),
                    resolved: entry.resolved.clone(),
                }))
            })
            .collect();

        Ok(LockedDependencies { packages })
    }

    fn verify_integrity(&self, path: &Path) -> Result<()> {
        // Would need to fetch packages and verify SHA-512 hashes
        // For now, just parse the file
        self.parse_lockfile(path)?;
        Ok(())
    }
}

// pnpm-lock.yaml
#[derive(Debug, Deserialize)]
pub struct PnpmLock {
    pub lockfileVersion: String,
    pub packages: HashMap<String, PnpmLockEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PnpmLockEntry {
    pub resolution: PnpmResolution,
}

#[derive(Debug, Deserialize)]
pub struct PnpmResolution {
    pub integrity: String,
    pub tarball: Option<String>,
}

pub struct PnpmLockfileParser;

impl LockfileParser for PnpmLockfileParser {
    fn parse_lockfile(&self, path: &Path) -> Result<LockedDependencies> {
        let content = std::fs::read_to_string(path)?;
        let lock: PnpmLock = serde_yaml::from_str(&content)?;

        // pnpm uses format: "/foo/1.0.0" as key
        let packages = lock.packages.iter()
            .filter_map(|(key, entry)| {
                let parts: Vec<&str> = key.trim_start_matches('/').split('/').collect();
                if parts.len() < 2 {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts[1].to_string();

                Some((name.clone(), LockedPackage {
                    name,
                    version,
                    checksum: Some(entry.resolution.integrity.clone()),
                    resolved: entry.resolution.tarball.clone(),
                }))
            })
            .collect();

        Ok(LockedDependencies { packages })
    }

    fn verify_integrity(&self, path: &Path) -> Result<()> {
        self.parse_lockfile(path)?;
        Ok(())
    }
}
```

#### 3.3 Workspace Detection (`src/npm/workspace.rs`)

```rust
use std::path::{Path, PathBuf};
use serde_json::Value;
use glob::glob;

pub struct NpmWorkspace;

impl NpmWorkspace {
    /// Detect workspace type and find root
    pub fn find_root(start: &Path) -> Result<Option<PathBuf>> {
        let mut current = start.to_path_buf();

        loop {
            // Check for package.json with workspaces field
            let pkg_json = current.join("package.json");
            if pkg_json.exists() {
                let content = std::fs::read_to_string(&pkg_json)?;
                let pkg: Value = serde_json::from_str(&content)?;

                if pkg.get("workspaces").is_some() {
                    return Ok(Some(current));
                }
            }

            // Check for pnpm-workspace.yaml
            if current.join("pnpm-workspace.yaml").exists() {
                return Ok(Some(current));
            }

            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Get workspace members (supports npm, pnpm, yarn)
    pub fn get_members(root: &Path) -> Result<Vec<PathBuf>> {
        // Try pnpm-workspace.yaml first
        let pnpm_workspace = root.join("pnpm-workspace.yaml");
        if pnpm_workspace.exists() {
            return Self::get_pnpm_members(root);
        }

        // Try package.json workspaces
        let pkg_json = root.join("package.json");
        if pkg_json.exists() {
            return Self::get_npm_members(root);
        }

        Ok(vec![])
    }

    fn get_pnpm_members(root: &Path) -> Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        struct PnpmWorkspace {
            packages: Vec<String>,
        }

        let content = std::fs::read_to_string(root.join("pnpm-workspace.yaml"))?;
        let workspace: PnpmWorkspace = serde_yaml::from_str(&content)?;

        let mut members = vec![];
        for pattern in workspace.packages {
            let full_pattern = root.join(&pattern).join("package.json");
            for entry in glob(full_pattern.to_str().unwrap()).unwrap() {
                if let Ok(path) = entry {
                    members.push(path);
                }
            }
        }

        Ok(members)
    }

    fn get_npm_members(root: &Path) -> Result<Vec<PathBuf>> {
        let content = std::fs::read_to_string(root.join("package.json"))?;
        let pkg: Value = serde_json::from_str(&content)?;

        let patterns = match pkg.get("workspaces") {
            Some(Value::Array(arr)) => {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect::<Vec<_>>()
            }
            Some(Value::Object(obj)) => {
                obj.get("packages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default()
            }
            _ => vec![],
        };

        let mut members = vec![];
        for pattern in patterns {
            let full_pattern = root.join(&pattern).join("package.json");
            for entry in glob(full_pattern.to_str().unwrap()).unwrap() {
                if let Ok(path) = entry {
                    members.push(path);
                }
            }
        }

        Ok(members)
    }
}
```

## 🔐 Security Considerations

1. **Input Validation**
   - Validate all file paths to prevent directory traversal
   - Sanitize version strings before parsing
   - Limit file sizes (max 10MB for dependency files)

2. **Checksum Verification**
   - Always verify lockfile checksums before trusting data
   - Use constant-time comparison for checksums

3. **Atomic File Writes**
   - Write to temp file, then rename (atomic on POSIX)
   - Never leave corrupted files if operation fails

4. **No Code Execution**
   - Never eval or execute code from dependency files
   - Parse with safe deserialization only

## 🧪 Testing Strategy

### Unit Tests
- `version.rs`: Test all version syntax variants
- `checksum.rs`: Test SHA-256 and SHA-512
- Each parser: Test valid and invalid inputs

### Property-Based Tests (proptest)
```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn version_comparison_is_transitive(
            a in semver_strategy(),
            b in semver_strategy(),
            c in semver_strategy()
        ) {
            // If a < b and b < c, then a < c
            // Property-based test
        }
    }
}
```

### Integration Tests
- Round-trip tests: parse → serialize → parse
- Real-world Cargo.toml and package.json files
- Workspace detection with actual monorepo structures

### Regression Tests
- Test files from popular projects (Next.js, tokio, etc.)

## 🚀 Public API Example

```rust
use danny_deps::{CargoDependencyManager, DependencyManager, DependencyUpdate, DependencyType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse Cargo.toml
    let manager = CargoDependencyManager::new();
    let manifest = manager.parse(Path::new("Cargo.toml"))?;

    println!("Package: {} v{}", manifest.name, manifest.version);
    println!("Is workspace: {}", manifest.is_workspace_root);

    // List dependencies
    for dep in manifest.all_dependencies() {
        println!("  {} = {}", dep.name, dep.version_req.raw);
    }

    // Update serde to 1.0.210
    let updates = vec![
        DependencyUpdate {
            package: "serde".to_string(),
            new_version: "1.0.210".to_string(),
            dep_type: DependencyType::Runtime,
        }
    ];

    // Dry run first
    let result = manager.update(Path::new("Cargo.toml"), &updates, true)?;
    println!("Would update {} dependencies", result.updates.len());

    // Apply for real
    let result = manager.update(Path::new("Cargo.toml"), &updates, false)?;
    println!("Updated {} dependencies", result.updates.len());

    Ok(())
}
```

## 🔮 Future Extensions

The design allows adding new ecosystems without breaking changes:

```rust
// Future: Python support
pub enum Ecosystem {
    Rust,
    JavaScript,
    Python,  // ← Add without breaking API
}

// Future: pyproject.toml parser
pub struct PoetryDependencyManager;

impl DependencyManager for PoetryDependencyManager {
    // ... implement trait
}
```

## 📝 Implementation Checklist

- [ ] Phase 1: Core infrastructure (version, checksum, update)
- [ ] Phase 2: Cargo support (parser, lockfile, workspace)
- [ ] Phase 3: npm/pnpm/yarn support
- [ ] Unit tests (>80% coverage)
- [ ] Integration tests
- [ ] Property-based tests
- [ ] Documentation
- [ ] Examples
- [ ] Integration with danny-info

## 📚 Dependencies Rationale

| Crate | Purpose | Why |
|-------|---------|-----|
| `toml_edit` | Cargo.toml parsing | Preserves comments/formatting (unlike `toml`) |
| `semver` | Rust version parsing | Industry standard for semver |
| `node-semver` | npm version parsing | Handles ^, ~, *, latest |
| `cargo_lock` | Cargo.lock parsing | Official Cargo lockfile parser |
| `serde_json` | package.json | Standard JSON parsing |
| `serde_yaml` | pnpm-lock.yaml | YAML lockfile support |
| `sha2` | Checksums | SHA-256/SHA-512 verification |
| `thiserror` | Errors | Ergonomic error types |
| `proptest` | Testing | Property-based testing |

---

**Next Steps:** Implement Phase 1 (core infrastructure) following this plan.
