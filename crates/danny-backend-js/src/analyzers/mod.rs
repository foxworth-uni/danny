//! Analyzers for Phase 1 features.
//!
//! Each analyzer focuses on a specific aspect of the module graph:
//! - `side_effects`: Safety assessment for deletion
//! - `bundle_size`: Bundle impact calculation
//! - `type_only`: TypeScript type-only categorization
//! - `dynamic_imports`: Dynamic import extraction
//! - `class_members`: Class member analysis
//! - `enum_members`: Enum member analysis
//! - `npm_dependencies`: NPM dependency analysis
//! - `dependency_chains`: Dependency chain analysis

pub mod bundle_size;
pub mod class_members;
pub mod dependency_chains;
pub mod dynamic_imports;
pub mod enum_members;
pub mod npm_dependencies;
pub mod quality;
pub mod side_effects;
pub mod type_only;

pub use bundle_size::BundleSizeAnalyzer;
pub use class_members::ClassMemberAnalyzer;
pub use dependency_chains::DependencyChainAnalyzer;
pub use dynamic_imports::DynamicImportAnalyzer;
pub use enum_members::EnumMemberAnalyzer;
pub use npm_dependencies::NpmDependencyAnalyzer;
pub use quality::QualityAnalyzer;
pub use side_effects::SideEffectAnalyzer;
pub use type_only::{CategorizedExports, TypeOnlyAnalyzer, UnusedExport};
