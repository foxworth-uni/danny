//! Danny Core - Language backend abstraction and core types.
//!
//! This crate provides the foundational types and traits for Danny's
//! language-agnostic analysis system. It defines:
//!
//! - [`LanguageBackend`]: Trait for implementing language-specific analyzers
//! - [`BackendRegistry`]: Registry for discovering and selecting backends
//! - [`Finding`]: Common representation of analysis results
//! - [`AnalysisOptions`] and [`AnalysisResult`]: Core analysis types
//!
//! # Architecture
//!
//! Danny uses a backend-based architecture where language-specific
//! functionality is delegated to backend implementations:
//!
//! ```text
//! ┌─────────────────┐
//! │   danny-cli     │  (User interface)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  danny-core     │  (This crate - backend abstraction)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │danny-backend-js │  (Fob-based JavaScript/TypeScript)
//! └─────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use danny_core::{BackendRegistry, AnalysisOptions};
//! use std::path::PathBuf;
//!
//! let mut registry = BackendRegistry::new();
//! // Register backends here
//!
//! let backend = registry.find_by_extension(".ts").expect("No TypeScript backend");
//!
//! let options = AnalysisOptions {
//!     entry_points: vec![PathBuf::from("src/index.ts")],
//!     project_root: PathBuf::from("."),
//!     ..Default::default()
//! };
//!
//! let result = backend.analyze(options)?;
//! println!("Found {} modules", result.statistics.total_modules);
//! # Ok::<(), danny_core::Error>(())
//! ```

pub mod backend;
pub mod circular_deps;
pub mod enrichment;
pub mod error;
pub mod types;
pub mod validation;

// Re-export core types for convenience
pub use backend::{BackendRegistry, LanguageBackend};
pub use error::{Error, Result};
pub use types::{
    AnalysisCapabilities, AnalysisError, AnalysisMode, AnalysisOptions, AnalysisResult, Category,
    ClassMemberKind, ClassMemberStats, Dependency, DependencyCoverageStats, EnumStats, EnumValue,
    ErrorSeverity, ExportKind, Finding, IgnoredFinding, IgnoredFindingsBreakdown, MemberVisibility,
    NpmDependencyType, PatternType, SafetyAssessment, SourceLocation, Statistics, SymbolSpan,
    TypeCoverage, UnavailableCategory, UnavailableReason,
};
