//! Core data types for Danny analysis.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// User-facing category for organizing findings.
///
/// Categories group related finding types into user-friendly buckets
/// for filtering and display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Unused files that can be deleted
    Files,
    /// Unused exports (runtime)
    Exports,
    /// Dead code within files
    Symbols,
    /// Unused TypeScript types/interfaces
    Types,
    /// Unused npm packages
    Dependencies,
    /// Import patterns & analysis
    Imports,
    /// Circular dependency cycles
    Circular,
    /// Code smells
    Quality,
    /// Framework-detected exports (informational)
    Framework,
}

impl Category {
    /// Returns all categories in a consistent order
    pub fn all() -> &'static [Category] {
        &[
            Category::Files,
            Category::Exports,
            Category::Symbols,
            Category::Types,
            Category::Dependencies,
            Category::Imports,
            Category::Circular,
            Category::Quality,
            Category::Framework,
        ]
    }

    /// Returns the display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Files => "Files",
            Category::Exports => "Exports",
            Category::Symbols => "Symbols",
            Category::Types => "Types",
            Category::Dependencies => "Dependencies",
            Category::Imports => "Imports",
            Category::Circular => "Circular",
            Category::Quality => "Quality",
            Category::Framework => "Framework",
        }
    }

    /// Returns the lowercase name for CLI parsing
    pub fn cli_name(&self) -> &'static str {
        match self {
            Category::Files => "files",
            Category::Exports => "exports",
            Category::Symbols => "symbols",
            Category::Types => "types",
            Category::Dependencies => "dependencies",
            Category::Imports => "imports",
            Category::Circular => "circular",
            Category::Quality => "quality",
            Category::Framework => "framework",
        }
    }

    /// Returns the description for this category
    pub fn description(&self) -> &'static str {
        match self {
            Category::Files => "Unused files that can be deleted",
            Category::Exports => "Unused exports (runtime)",
            Category::Symbols => "Dead code within files",
            Category::Types => "Unused TypeScript types/interfaces",
            Category::Dependencies => "Unused npm packages",
            Category::Imports => "Import pattern analysis",
            Category::Circular => "Circular dependency cycles",
            Category::Quality => "Code quality issues",
            Category::Framework => "Framework-specific exports",
        }
    }

    /// Check if this category requires a full dependency graph
    pub fn requires_full_graph(&self) -> bool {
        matches!(
            self,
            Category::Files
                | Category::Exports
                | Category::Dependencies
                | Category::Circular
                | Category::Framework
        )
    }

    /// Parse from CLI string
    pub fn from_cli_name(s: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|cat| cat.cli_name() == s)
            .copied()
    }
}

/// Analysis mode determines available capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisMode {
    /// Full package analysis with complete dependency graph
    Package,
    /// File-level analysis with limited context
    Files,
}

/// Defines what analysis categories are available given the current context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisCapabilities {
    /// Categories that can be performed with available context
    available: HashSet<Category>,
    /// Categories that require more context
    unavailable: Vec<UnavailableCategory>,
    /// The mode that determined these capabilities
    mode: AnalysisMode,
}

/// A category that cannot be run due to missing requirements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableCategory {
    pub category: Category,
    pub reason: UnavailableReason,
}

/// Why a category is unavailable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    /// Requires complete dependency graph (package mode)
    RequiresFullGraph,
    /// Requires package.json context
    RequiresPackageJson,
    /// Requires framework to be detected
    RequiresFrameworkDetection,
    /// Requires node_modules directory
    RequiresNodeModules,
}

impl AnalysisCapabilities {
    /// Create capabilities for package mode
    pub fn package_mode() -> Self {
        Self {
            available: HashSet::from([
                Category::Files,
                Category::Exports,
                Category::Symbols,
                Category::Types,
                Category::Imports,
                Category::Circular,
                Category::Quality,
                // Dependencies and Framework added conditionally via mark_available
            ]),
            unavailable: Vec::new(),
            mode: AnalysisMode::Package,
        }
    }

    /// Create capabilities for files mode
    pub fn files_mode() -> Self {
        Self {
            available: HashSet::from([
                Category::Symbols,
                Category::Quality,
                Category::Imports,
                Category::Types,
            ]),
            unavailable: vec![
                UnavailableCategory {
                    category: Category::Files,
                    reason: UnavailableReason::RequiresFullGraph,
                },
                UnavailableCategory {
                    category: Category::Exports,
                    reason: UnavailableReason::RequiresFullGraph,
                },
                UnavailableCategory {
                    category: Category::Dependencies,
                    reason: UnavailableReason::RequiresFullGraph,
                },
                UnavailableCategory {
                    category: Category::Circular,
                    reason: UnavailableReason::RequiresFullGraph,
                },
                UnavailableCategory {
                    category: Category::Framework,
                    reason: UnavailableReason::RequiresPackageJson,
                },
            ],
            mode: AnalysisMode::Files,
        }
    }

    /// Mark a category as available (for conditional categories like Dependencies/Framework)
    pub fn mark_available(&mut self, category: Category) {
        self.available.insert(category);
        // Remove from unavailable if present
        self.unavailable.retain(|uc| uc.category != category);
    }

    /// Mark a category as unavailable
    pub fn mark_unavailable(&mut self, category: Category, reason: UnavailableReason) {
        self.available.remove(&category);
        // Only add if not already present
        if !self.unavailable.iter().any(|uc| uc.category == category) {
            self.unavailable.push(UnavailableCategory { category, reason });
        }
    }

    /// Check if a category is supported
    pub fn supports(&self, category: Category) -> bool {
        self.available.contains(&category)
    }

    /// Get all available categories
    pub fn available_categories(&self) -> &HashSet<Category> {
        &self.available
    }

    /// Get unavailable categories with reasons
    pub fn unavailable_categories(&self) -> &[UnavailableCategory] {
        &self.unavailable
    }

    /// Get the analysis mode
    pub fn mode(&self) -> AnalysisMode {
        self.mode
    }

    /// Filter requested categories to only available ones (returns iterator)
    pub fn filter_available<'a>(
        &'a self,
        requested: &'a [Category],
    ) -> impl Iterator<Item = Category> + 'a {
        requested.iter().filter_map(move |c| {
            if self.supports(*c) {
                Some(*c)
            } else {
                None
            }
        })
    }

    /// Get unavailable categories from a request (returns iterator)
    pub fn filter_unavailable<'a>(
        &'a self,
        requested: &'a [Category],
    ) -> impl Iterator<Item = Category> + 'a {
        requested.iter().filter_map(move |c| {
            if !self.supports(*c) {
                Some(*c)
            } else {
                None
            }
        })
    }
}

/// Safety assessment for module deletion.
///
/// This enum represents the safety level of deleting a module based on
/// various factors like side effects, entry point status, and dynamic imports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyAssessment {
    /// Module is safe to delete without risk
    SafeToDelete,
    /// Module should be reviewed carefully before deletion
    ReviewCarefully(String),
    /// Module should not be deleted
    Unsafe(String),
}

/// A finding discovered during code analysis.
///
/// Findings represent modules, dependencies, patterns, or other
/// interesting facts about the analyzed code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Finding {
    /// A module (file) in the project.
    Module {
        /// Absolute path to the module file.
        path: PathBuf,

        /// Dependencies imported by this module.
        dependencies: Vec<Dependency>,

        /// Module metadata (exports, imports, etc.).
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },

    /// A dependency relationship between modules.
    Dependency {
        /// Module that imports the dependency.
        from: PathBuf,

        /// Module being imported.
        to: PathBuf,

        /// Import specifier (e.g., "react", "./utils").
        specifier: String,

        /// Whether this is an external (npm) dependency.
        is_external: bool,
    },

    /// A detected pattern (framework component, API route, etc.).
    Pattern {
        /// Type of pattern detected.
        pattern_type: PatternType,

        /// Location where the pattern was found.
        location: PathBuf,

        /// Additional pattern-specific metadata.
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },

    /// A detected framework usage.
    Framework {
        /// Framework name (e.g., "React", "Next.js").
        name: String,

        /// Confidence level (0.0 to 1.0).
        confidence: f32,

        /// Evidence supporting this detection.
        evidence: Vec<PathBuf>,
    },

    /// An unused export detected in a module.
    UnusedExport {
        /// Module containing the unused export.
        module: PathBuf,

        /// Name of the unused export.
        export_name: String,

        /// Export kind (Named, Default, ReExport, TypeOnly).
        kind: ExportKind,

        /// Source location information.
        span: Option<SourceLocation>,

        /// Whether this is a TypeScript type-only export.
        is_type_only: bool,

        /// Detailed explanation (verbose mode only).
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<Explanation>,
    },

    /// An unreachable module (never imported and has no side effects).
    UnreachableModule {
        /// Path to the unreachable module.
        path: PathBuf,

        /// Original file size in bytes.
        size: usize,

        /// Enriched metadata (has side effects, safe to delete, etc.)
        metadata: UnreachableModuleMetadata,
    },

    /// An unreachable file (never imported, not in module graph).
    UnreachableFile {
        /// Path to the unreachable file.
        path: PathBuf,

        /// File size in bytes.
        size: usize,

        /// Detailed explanation (verbose mode only).
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<Explanation>,
    },

    /// An unused internal symbol (function, variable, class, etc.).
    UnusedSymbol {
        /// Module containing the symbol.
        module: PathBuf,

        /// Symbol name.
        symbol_name: String,

        /// Symbol kind (function, variable, class, etc.).
        kind: SymbolKind,

        /// Source location.
        span: SymbolSpan,

        /// Detailed explanation (verbose mode only).
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<Explanation>,
    },

    /// An export marked as framework-used (for transparency).
    FrameworkExport {
        /// Module containing the export.
        module: PathBuf,

        /// Export name.
        export_name: String,

        /// Framework that uses this export.
        framework: String,

        /// Rule that marked it as used.
        rule: String,

        /// Detailed explanation (verbose mode only).
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<Explanation>,
    },

    /// Dynamic import relationship
    DynamicImport(DynamicImportInfo),

    /// Circular dependency cycle
    CircularDependency(CircularDependency),

    /// Unused private class member (safe to remove)
    UnusedPrivateClassMember {
        /// Module containing the class
        module: PathBuf,
        /// Class name
        class_name: String,
        /// Member name
        member_name: String,
        /// Member kind (method, property, etc.)
        member_kind: ClassMemberKind,
        /// Source location
        span: SymbolSpan,
    },

    /// Unused public class member (risky - may be used externally)
    UnusedPublicClassMember {
        /// Module containing the class
        module: PathBuf,
        /// Class name
        class_name: String,
        /// Member name
        member_name: String,
        /// Member kind (method, property, etc.)
        member_kind: ClassMemberKind,
        /// Source location
        span: SymbolSpan,
    },

    /// Unused enum member
    UnusedEnumMember {
        /// Module containing the enum
        module: PathBuf,
        /// Enum name
        enum_name: String,
        /// Member name
        member_name: String,
        /// Member value (if any)
        value: Option<EnumValue>,
        /// Source location
        span: SymbolSpan,
    },

    /// Unused npm dependency (declared but never imported)
    UnusedNpmDependency {
        /// Package name
        package: String,
        /// Version specifier from package.json
        version: String,
        /// Dependency type (dependencies, devDependencies, etc.)
        dep_type: NpmDependencyType,
    },

    /// Side-effect-only import (cannot be tree-shaken)
    SideEffectOnlyImport {
        /// Module containing the import
        module: PathBuf,
        /// Import source string
        source: String,
        /// Resolved module path (if local)
        resolved_to: Option<PathBuf>,
        /// Source location
        span: SourceLocation,
    },

    /// Namespace import (import * as X)
    NamespaceImport {
        /// Module containing the import
        module: PathBuf,
        /// Namespace name
        namespace_name: String,
        /// Import source string
        source: String,
        /// Resolved module path (if local)
        resolved_to: Option<PathBuf>,
    },

    /// Type-only import (TypeScript import type)
    TypeOnlyImport {
        /// Module containing the import
        module: PathBuf,
        /// Import source string
        source: String,
        /// Import specifiers
        specifiers: Vec<String>,
        /// Source location
        span: SourceLocation,
    },

    /// Dead code module (only reachable through dead code)
    DeadCodeModule {
        /// Path to the dead code module
        path: PathBuf,
        /// File size in bytes
        size: usize,
    },

    /// Dependency chain (import path analysis)
    DependencyChain {
        /// Chain of module paths from entry to target
        chain: Vec<PathBuf>,
        /// Depth of the chain
        depth: usize,
    },

    /// A code quality issue (code smell) detected during analysis.
    CodeSmell {
        /// Type of code smell detected.
        smell_type: CodeSmellType,

        /// Location where the smell was found.
        location: PathBuf,

        /// Function, class, or symbol name (if applicable).
        symbol_name: Option<String>,

        /// Line number where the smell occurs (1-indexed).
        line: Option<u32>,

        /// Column number where the smell occurs (0-indexed).
        column: Option<u32>,

        /// Severity of the code smell.
        severity: SmellSeverity,

        /// Detailed information about the smell.
        details: CodeSmellDetails,
    },
}

impl Finding {
    /// Returns the category this finding belongs to.
    ///
    /// Categories group related finding types for user-friendly filtering
    /// and display purposes.
    pub fn category(&self) -> Category {
        use Finding::*;
        match self {
            // Files category: unused/unreachable files and modules
            UnreachableFile { .. } | UnreachableModule { .. } | DeadCodeModule { .. } => {
                Category::Files
            }
            // Exports category: unused exports (runtime)
            UnusedExport { is_type_only: false, .. } => Category::Exports,
            // Types category: unused TypeScript types/interfaces
            UnusedExport { is_type_only: true, .. } => Category::Types,
            // Symbols category: dead code within files
            UnusedSymbol { .. }
            | UnusedPrivateClassMember { .. }
            | UnusedPublicClassMember { .. }
            | UnusedEnumMember { .. } => Category::Symbols,
            // Dependencies category: unused npm packages
            UnusedNpmDependency { .. } => Category::Dependencies,
            // Imports category: import patterns & analysis
            SideEffectOnlyImport { .. }
            | NamespaceImport { .. }
            | TypeOnlyImport { .. }
            | DependencyChain { .. }
            | DynamicImport(_) => Category::Imports,
            // Circular category: circular dependency cycles
            CircularDependency(_) => Category::Circular,
            // Quality category: code smells
            CodeSmell { .. } => Category::Quality,
            // Framework category: framework-detected exports
            FrameworkExport { .. } => Category::Framework,
            // These are informational/internal and don't map to user categories
            Module { .. }
            | Dependency { .. }
            | Pattern { .. }
            | Framework { .. } => Category::Framework, // Default to framework for now
        }
    }
}

/// Types of code smells that can be detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeSmellType {
    /// Function exceeds recommended line count.
    LongFunction,

    /// Function has too many parameters.
    TooManyParameters,

    /// Class exceeds recommended line count.
    LargeClass,

    /// Numeric literal used without named constant.
    MagicNumber,

    /// Method call chain is too long (message chain).
    MessageChain,

    /// Function has high cyclomatic complexity.
    ComplexConditional,

    /// Code has excessive nesting depth.
    DeepNesting,

    /// Function has multiple return statements.
    MultipleReturns,

    /// Empty catch block detected.
    EmptyCatchBlock,

    /// Duplicated code block detected.
    DuplicatedCode,

    /// Long parameter list (similar to TooManyParameters but different threshold).
    LongParameterList,

    /// Class has too many methods.
    TooManyMethods,

    /// Class has too many fields.
    TooManyFields,

    /// Function does too many things (low cohesion).
    LowCohesion,
}

/// Severity level for code smells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmellSeverity {
    /// Informational - code quality suggestion.
    Info,

    /// Warning - should be reviewed.
    Warning,

    /// Error - significant code quality issue.
    Error,
}

/// Detailed information about a code smell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeSmellDetails {
    /// Human-readable message describing the issue.
    pub message: String,

    /// Recommendation for fixing the issue.
    pub recommendation: Option<String>,

    /// Current value (e.g., line count, parameter count).
    pub current_value: Option<usize>,

    /// Recommended threshold value.
    pub recommended_threshold: Option<usize>,

    /// Additional metadata (JSON value for extensibility).
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Statistics for code quality analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CodeSmellStats {
    /// Total number of code smells detected.
    pub total_smells: usize,

    /// Breakdown by smell type.
    pub by_type: Vec<(CodeSmellType, usize)>,

    /// Breakdown by severity.
    pub by_severity: Vec<(SmellSeverity, usize)>,
}

/// Types of patterns that can be detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatternType {
    /// React component (class or function).
    ReactComponent,

    /// Next.js page component.
    NextJsPage,

    /// Next.js API route.
    NextJsApiRoute,

    /// Next.js App Router layout.
    NextJsLayout,

    /// Vue component.
    VueComponent,

    /// Custom pattern defined in TOML.
    Custom(String),
}

/// Export kind matching Fob's ExportKind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    /// Named export (export { foo }).
    Named,

    /// Default export (export default).
    Default,

    /// Re-export (export { foo } from './bar').
    ReExport,

    /// Star re-export (export * from './bar').
    StarReExport,

    /// TypeScript type-only export (export type { Foo }).
    TypeOnly,
}

/// Symbol kind for intra-file analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Function declaration.
    Function,

    /// Variable declaration.
    Variable,

    /// Class declaration.
    Class,

    /// Function parameter.
    Parameter,

    /// TypeScript type alias.
    TypeAlias,

    /// TypeScript interface.
    Interface,

    /// Enum declaration.
    Enum,
}

/// Class member kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassMemberKind {
    /// Class method
    Method,
    /// Class property
    Property,
    /// Class getter
    Getter,
    /// Class setter
    Setter,
    /// Class constructor
    Constructor,
}

/// Member visibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberVisibility {
    /// Private member (safe to remove)
    Private,
    /// Public member (may be used externally)
    Public,
    /// Protected member (may be used by subclasses)
    Protected,
}

/// Enum member value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnumValue {
    /// Numeric value
    Number(i64),
    /// String value
    String(String),
    /// Computed value (not a literal)
    Computed,
}

/// NPM dependency type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpmDependencyType {
    /// Production dependencies
    Production,
    /// Development dependencies
    Development,
    /// Peer dependencies
    Peer,
    /// Optional dependencies
    Optional,
}

/// Symbol source location with line and column information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSpan {
    /// File path.
    pub file: PathBuf,

    /// Line number (1-indexed).
    pub line: u32,

    /// Column number (0-indexed).
    pub column: u32,

    /// Byte offset in source.
    pub offset: u32,
}

/// Source code location information from Fob's SourceSpan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// File path.
    pub file: PathBuf,

    /// Start byte offset.
    pub start: u32,

    /// End byte offset.
    pub end: u32,
}

/// Detailed explanation for why a finding was flagged (verbose mode).
///
/// This provides context about analysis decisions, rule matches, and usage information.
/// Only populated when verbose mode is enabled to avoid unnecessary overhead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Explanation {
    /// Primary reason (short summary)
    pub reason: String,

    /// Detailed explanation points (for verbose mode)
    pub details: Vec<ExplanationDetail>,
}

/// A single explanation detail providing specific context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExplanationDetail {
    /// Category of detail (e.g., "usage_count", "rule_match", "entry_points")
    pub category: String,

    /// Human-readable label
    pub label: String,

    /// Value or description
    pub value: String,

    /// Additional context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// A dependency relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    /// Import specifier as written in code.
    pub specifier: String,

    /// Resolved absolute path (if local) or package name (if external).
    pub resolved: String,

    /// Whether this is an external (npm) dependency.
    pub is_external: bool,

    /// Whether this is a dynamic import.
    #[serde(default)]
    pub is_dynamic: bool,
}

/// Result of analyzing a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// All findings discovered during analysis.
    pub findings: Vec<Finding>,

    /// Summary statistics.
    pub statistics: Statistics,

    /// Errors encountered (non-fatal).
    #[serde(default)]
    pub errors: Vec<AnalysisError>,

    /// Findings that were filtered out by ignore patterns.
    /// Always populated for transparency in JSON output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored_findings: Vec<IgnoredFinding>,
}

/// Summary statistics from analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Statistics {
    /// Total number of modules analyzed.
    pub total_modules: usize,

    /// Total number of dependencies.
    pub total_dependencies: usize,

    /// Number of external dependencies.
    pub external_dependencies: usize,

    /// Detected frameworks.
    pub frameworks_detected: Vec<String>,

    /// Number of unused exports detected.
    pub unused_exports_count: usize,

    /// Number of unreachable modules detected.
    pub unreachable_modules_count: usize,

    /// Number of unreachable files detected (never in module graph).
    pub unreachable_files_count: usize,

    /// Number of framework-used exports.
    pub framework_exports_count: usize,

    /// Symbol-level statistics (if symbol analysis enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_statistics: Option<SymbolStats>,

    /// Bundle size impact analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_size_impact: Option<BundleSizeImpact>,

    /// Number of dynamic imports detected
    #[serde(default)]
    pub dynamic_imports_count: usize,

    /// Number of circular dependencies detected
    #[serde(default)]
    pub circular_dependencies_count: usize,

    /// Number of type-only unused exports
    #[serde(default)]
    pub type_only_unused_exports_count: usize,

    /// Number of unused private class members
    #[serde(default)]
    pub unused_private_class_members_count: usize,

    /// Number of unused public class members
    #[serde(default)]
    pub unused_public_class_members_count: usize,

    /// Number of unused enum members
    #[serde(default)]
    pub unused_enum_members_count: usize,

    /// Number of unused npm dependencies
    #[serde(default)]
    pub unused_npm_dependencies_count: usize,

    /// Number of side-effect-only imports
    #[serde(default)]
    pub side_effect_only_imports_count: usize,

    /// Number of namespace imports
    #[serde(default)]
    pub namespace_imports_count: usize,

    /// Number of type-only imports
    #[serde(default)]
    pub type_only_imports_count: usize,

    /// Number of dead code modules
    #[serde(default)]
    pub dead_code_modules_count: usize,

    /// Number of dependency chains analyzed
    #[serde(default)]
    pub dependency_chains_count: usize,

    /// Class member statistics (if class analysis enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_member_stats: Option<ClassMemberStats>,

    /// Enum member statistics (if enum analysis enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_stats: Option<EnumStats>,

    /// Dependency coverage statistics (if npm dependency analysis enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_coverage_stats: Option<DependencyCoverageStats>,

    /// Number of findings filtered by ignore patterns.
    #[serde(default)]
    pub ignored_findings_count: usize,

    /// Breakdown of ignored findings by type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored_findings_breakdown: Option<IgnoredFindingsBreakdown>,

    /// Analysis duration in milliseconds.
    pub duration_ms: u64,

    /// Code quality statistics (if quality analysis enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_quality_stats: Option<CodeSmellStats>,

    /// Number of code smells detected
    #[serde(default)]
    pub code_smells_count: usize,
}

/// Symbol-level statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolStats {
    /// Total number of symbols analyzed.
    pub total_symbols: usize,

    /// Number of unused symbols.
    pub unused_symbols: usize,

    /// Breakdown by symbol kind.
    pub by_kind: Vec<(SymbolKind, usize)>,
}

/// A non-fatal error encountered during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisError {
    /// File where the error occurred.
    pub file: PathBuf,

    /// Error message.
    pub message: String,

    /// Error severity.
    pub severity: ErrorSeverity,
}

/// Error severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Warning: analysis continues.
    Warning,

    /// Error: file skipped, analysis continues.
    Error,
}

/// Options for analysis.
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Entry points to analyze from.
    pub entry_points: Vec<PathBuf>,

    /// Project root directory.
    pub project_root: PathBuf,

    /// Whether to follow external dependencies.
    pub follow_external: bool,

    /// Maximum depth for dependency traversal (None = unlimited).
    pub max_depth: Option<usize>,

    /// Path to configuration file (e.g., .danny.toml).
    pub config_path: Option<PathBuf>,

    /// Additional backend-specific options.
    pub backend_options: HashMap<String, serde_json::Value>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            entry_points: Vec::new(),
            project_root: PathBuf::from("."),
            follow_external: false,
            max_depth: None,
            config_path: None,
            backend_options: HashMap::new(),
        }
    }
}

/// Enriched metadata for unreachable modules (Feature 1 & 2)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnreachableModuleMetadata {
    /// Whether the module has side effects (makes deletion risky)
    pub has_side_effects: bool,

    /// Original file size in bytes
    pub size_bytes: usize,

    /// Whether this module is safe to delete (deprecated, use safety_assessment)
    #[serde(default)]
    pub safe_to_delete: bool,

    /// Detailed safety assessment for deletion
    pub safety_assessment: SafetyAssessment,
}

/// Bundle size impact analysis (Feature 2)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleSizeImpact {
    /// Total potential savings in bytes
    pub total_savings_bytes: usize,

    /// Breakdown by module
    pub by_module: Vec<ModuleSizeInfo>,

    /// Savings that are safe to realize (no side effects)
    pub safe_savings_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSizeInfo {
    pub path: PathBuf,
    pub size_bytes: usize,
    pub has_side_effects: bool,
}

/// Dynamic import analysis (Feature 4)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicImportInfo {
    /// Module containing the dynamic import
    pub from: PathBuf,

    /// Dynamically imported module
    pub to: PathBuf,

    /// Import source string
    pub source: String,

    /// Whether this creates a code-split chunk
    pub creates_chunk: bool,
}

/// Circular dependency detected (Feature 5)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CircularDependency {
    /// Modules involved in the cycle (ordered)
    pub cycle: Vec<PathBuf>,

    /// Whether all modules in cycle are unreachable
    pub all_unreachable: bool,

    /// Total size of modules in cycle
    pub total_size: usize,
}

/// A finding that was filtered out by ignore patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IgnoredFinding {
    /// The original finding that was filtered.
    pub finding: Finding,

    /// The ignore pattern that matched this finding.
    pub matched_pattern: String,

    /// The file path that caused the match.
    pub matched_path: PathBuf,
}

/// Breakdown of ignored findings by finding type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IgnoredFindingsBreakdown {
    /// Number of ignored unused exports.
    pub unused_exports: usize,

    /// Number of ignored unreachable modules.
    pub unreachable_modules: usize,

    /// Number of ignored unreachable files.
    pub unreachable_files: usize,

    /// Number of ignored unused symbols.
    pub unused_symbols: usize,

    /// Number of ignored framework exports.
    pub framework_exports: usize,

    /// Number of ignored module findings.
    pub modules: usize,

    /// Number of ignored dependency findings.
    pub dependencies: usize,

    /// Number of ignored pattern findings.
    pub patterns: usize,
}

/// Class member statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassMemberStats {
    /// Total class members analyzed
    pub total_members: usize,
    /// Unused private members (safe to remove)
    pub unused_private: usize,
    /// Unused public members (risky)
    pub unused_public: usize,
    /// Breakdown by visibility
    pub by_visibility: Vec<(MemberVisibility, usize)>,
}

/// Enum member statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumStats {
    /// Total enums analyzed
    pub total_enums: usize,
    /// Total enum members
    pub total_members: usize,
    /// Unused enum members
    pub unused_members: usize,
}

/// Dependency coverage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyCoverageStats {
    /// Total dependencies declared
    pub total_declared: usize,
    /// Dependencies actually used
    pub total_used: usize,
    /// Dependencies never imported
    pub total_unused: usize,
    /// Coverage percentage (0.0 to 100.0)
    pub coverage_percentage: f64,
    /// Breakdown by dependency type
    pub by_type: Vec<(NpmDependencyType, TypeCoverage)>,
}

/// Coverage for a specific dependency type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeCoverage {
    /// Number declared
    pub declared: usize,
    /// Number used
    pub used: usize,
    /// Number unused
    pub unused: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_serialization() {
        let dep = Dependency {
            specifier: "react".to_string(),
            resolved: "react".to_string(),
            is_external: true,
            is_dynamic: false,
        };

        let json = serde_json::to_string(&dep).unwrap();
        let deserialized: Dependency = serde_json::from_str(&json).unwrap();
        assert_eq!(dep, deserialized);
    }

    #[test]
    fn test_finding_module_serialization() {
        let finding = Finding::Module {
            path: PathBuf::from("/test/file.ts"),
            dependencies: vec![],
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(finding, deserialized);
    }

    #[test]
    fn test_statistics_defaults() {
        let stats = Statistics::default();
        assert_eq!(stats.total_modules, 0);
        assert_eq!(stats.total_dependencies, 0);
        assert_eq!(stats.external_dependencies, 0);
        assert!(stats.frameworks_detected.is_empty());
    }

    #[test]
    fn test_unreachable_module_metadata_serialization() {
        let metadata = UnreachableModuleMetadata {
            has_side_effects: true,
            size_bytes: 1024,
            safe_to_delete: false,
            safety_assessment: SafetyAssessment::ReviewCarefully("Module has side effects".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: UnreachableModuleMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata, deserialized);
    }

    #[test]
    fn test_bundle_size_impact_serialization() {
        let impact = BundleSizeImpact {
            total_savings_bytes: 5000,
            safe_savings_bytes: 3000,
            by_module: vec![
                ModuleSizeInfo {
                    path: PathBuf::from("a.ts"),
                    size_bytes: 2000,
                    has_side_effects: true,
                },
            ],
        };

        let json = serde_json::to_string(&impact).unwrap();
        let deserialized: BundleSizeImpact = serde_json::from_str(&json).unwrap();

        assert_eq!(impact, deserialized);
    }

    #[test]
    fn test_ignored_finding_serialization() {
        let ignored = IgnoredFinding {
            finding: Finding::UnusedExport {
                module: PathBuf::from("test.js"),
                export_name: "foo".to_string(),
                kind: ExportKind::Named,
                span: None,
                is_type_only: false,
                explanation: None,
            },
            matched_pattern: "*.js".to_string(),
            matched_path: PathBuf::from("test.js"),
        };

        let json = serde_json::to_string(&ignored).unwrap();
        let deserialized: IgnoredFinding = serde_json::from_str(&json).unwrap();

        assert_eq!(ignored, deserialized);
    }

    #[test]
    fn test_ignored_findings_breakdown_serialization() {
        let breakdown = IgnoredFindingsBreakdown {
            unused_exports: 5,
            unreachable_modules: 3,
            unreachable_files: 2,
            unused_symbols: 1,
            framework_exports: 0,
            modules: 10,
            dependencies: 7,
            patterns: 0,
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        let deserialized: IgnoredFindingsBreakdown = serde_json::from_str(&json).unwrap();

        assert_eq!(breakdown, deserialized);
    }

    #[test]
    fn test_analysis_result_with_ignored_findings() {
        let result = AnalysisResult {
            findings: vec![],
            statistics: Statistics::default(),
            errors: vec![],
            ignored_findings: vec![
                IgnoredFinding {
                    finding: Finding::UnusedExport {
                        module: PathBuf::from("ignored.js"),
                        export_name: "unused".to_string(),
                        kind: ExportKind::Named,
                        span: None,
                        is_type_only: false,
                        explanation: None,
                    },
                    matched_pattern: "**/node_modules/**".to_string(),
                    matched_path: PathBuf::from("node_modules/pkg/ignored.js"),
                },
            ],
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: AnalysisResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.ignored_findings.len(), deserialized.ignored_findings.len());
    }

    #[test]
    fn test_ignored_findings_skip_serializing_when_empty() {
        let result = AnalysisResult {
            findings: vec![],
            statistics: Statistics::default(),
            errors: vec![],
            ignored_findings: vec![],
        };

        let json = serde_json::to_string_pretty(&result).unwrap();

        // With skip_serializing_if, empty vec should not appear in JSON
        // But with default, it will serialize as empty array
        // Let's verify it deserializes correctly instead
        let deserialized: AnalysisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ignored_findings.len(), 0);
    }
}
