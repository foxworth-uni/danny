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

pub mod side_effects;
pub mod bundle_size;
pub mod type_only;
pub mod dynamic_imports;
pub mod class_members;
pub mod enum_members;
pub mod npm_dependencies;
pub mod dependency_chains;
pub mod quality;

pub use side_effects::SideEffectAnalyzer;
pub use bundle_size::BundleSizeAnalyzer;
pub use type_only::{TypeOnlyAnalyzer, CategorizedExports, UnusedExport};
pub use dynamic_imports::DynamicImportAnalyzer;
pub use class_members::ClassMemberAnalyzer;
pub use enum_members::EnumMemberAnalyzer;
pub use npm_dependencies::NpmDependencyAnalyzer;
pub use dependency_chains::DependencyChainAnalyzer;
pub use quality::QualityAnalyzer;

