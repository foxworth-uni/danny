//! # danny-deps
//!
//! Local dependency file parsing and management for Rust and JavaScript projects.
//!
//! This crate provides functionality to:
//! - Parse dependency files (Cargo.toml, package.json)
//! - Parse lockfiles (Cargo.lock, package-lock.json, pnpm-lock.yaml, yarn.lock)
//! - Compare versions using semver and npm-style versioning
//! - Safely update dependency files while preserving formatting and comments
//! - Support monorepo/workspace scenarios (Cargo workspaces, pnpm/npm workspaces)
//! - Verify lockfile integrity (checksums)
//!
//! ## Architecture
//!
//! The crate follows a trait-based architecture inspired by `danny-core`:
//! - Core traits for extensibility (`DependencyManager`, `LockfileParser`)
//! - Ecosystem-specific implementations (Cargo, npm/pnpm/yarn)
//! - Integration with `danny-info` for fetching remote package data
//!
//! ## Example
//!
//! ```rust,no_run
//! use danny_deps::{CargoDependencyManager, DependencyManager};
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let manager = CargoDependencyManager::new();
//! let manifest = manager.parse(Path::new("Cargo.toml"))?;
//!
//! // Check what dependencies are present
//! for dep in manifest.dependencies() {
//!     println!("{}: {}", dep.name, dep.version_req);
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod cargo;
pub mod checksum;
pub mod error;
pub mod integration;
pub mod npm;
pub mod traits;
pub mod types;
pub mod update;
pub mod version;

// Re-export main types and traits
pub use error::{Error, Result};
pub use traits::{DependencyManager, LockedDependencies, LockedPackage, LockfileParser};
pub use types::{
    AppliedUpdate, Dependency, DependencyFile, DependencyType, DependencyUpdate, Ecosystem,
    UpdateResult, VersionReq,
};

// Re-export ecosystem-specific managers
pub use cargo::CargoDependencyManager;
pub use npm::NpmDependencyManager;

// Re-export version utilities
pub use version::{compare_versions, update_type, ParsedVersionReq, UpdateType};

// Re-export integration types
pub use integration::{UnifiedDependencyManager, UpdateRecommendation};
