//! Code quality analyzer - detects code smells using Fob's symbol data.

use crate::toml_config::CodeQualityConfig;
use danny_core::types::{CodeSmellDetails, CodeSmellType, SmellSeverity};
use danny_core::Finding;
use fob::graph::{ModuleGraph, SymbolKind, UnusedSymbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Virtual path prefix used by bundlers for synthetic modules.
///
/// Modules with paths starting with this prefix should be filtered out
/// from quality analysis as they don't represent real source code.
const VIRTUAL_PATH_PREFIX: &str = "virtual:";

/// Errors that can occur during code quality analysis.
#[derive(Error, Debug)]
pub enum QualityAnalysisError {
    /// Failed to retrieve symbols from the module graph.
    #[error("Failed to get symbols: {0}")]
    SymbolAccess(String),

    /// Failed to retrieve a specific module from the graph.
    #[error("Failed to get module {module_id}: {source}")]
    ModuleAccess {
        /// The ID of the module that couldn't be accessed.
        module_id: String,
        /// The underlying error message.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A module referenced by a symbol was not found in the graph.
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
}

/// Analyzer for code quality issues (code smells).
pub struct QualityAnalyzer;

impl QualityAnalyzer {
    /// Detects long functions that exceed the configured line count threshold.
    ///
    /// This analyzer examines all function symbols in the module graph and flags
    /// functions that exceed `config.max_function_lines` as code smells.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing long functions, or an error if symbol
    /// access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    ///
    /// # Example
    /// ```no_run
    /// # use danny_backend_js::analyzers::QualityAnalyzer;
    /// # use danny_backend_js::toml_config::CodeQualityConfig;
    /// # async fn example(graph: &fob::graph::ModuleGraph) {
    /// let config = CodeQualityConfig::default();
    /// let findings = QualityAnalyzer::detect_long_functions(graph, &config).await.unwrap();
    /// # }
    /// ```
    pub async fn detect_long_functions(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Function,
            config.max_function_lines,
            CodeSmellType::LongFunction,
            "Extract logical sections into separate functions",
            |symbol_info| symbol_info.symbol.line_count(),
            |name, value, threshold| {
                format!(
                    "Function '{}' has {} lines (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects functions with too many parameters.
    ///
    /// This analyzer examines all function symbols and flags those with more
    /// parameters than `config.max_parameters` as code smells. Functions with
    /// many parameters are harder to understand and maintain.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing functions with too many parameters,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    pub async fn detect_too_many_parameters(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Function,
            config.max_parameters,
            CodeSmellType::TooManyParameters,
            "Use options object to reduce parameter count",
            |symbol_info| symbol_info.symbol.parameter_count(),
            |name, value, threshold| {
                format!(
                    "Function '{}' has {} parameters (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects large classes that exceed the configured line count threshold.
    ///
    /// This analyzer examines all class symbols and flags those with more lines
    /// than `config.max_class_lines` as code smells. Large classes often violate
    /// the Single Responsibility Principle and are harder to maintain.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing large classes, or an error if symbol
    /// access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    pub async fn detect_large_classes(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Class,
            config.max_class_lines,
            CodeSmellType::LargeClass,
            "Consider splitting into smaller, focused classes",
            |symbol_info| symbol_info.symbol.line_count(),
            |name, value, threshold| {
                format!(
                    "Class '{}' has {} lines (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects classes with too many methods.
    ///
    /// Classes with many methods often violate the Single Responsibility Principle.
    /// They should be split into smaller, more focused classes or use composition.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing classes with too many methods,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    pub async fn detect_too_many_methods(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Class,
            config.max_methods,
            CodeSmellType::TooManyMethods,
            "Consider splitting into smaller classes or using composition",
            |symbol_info| symbol_info.symbol.method_count(),
            |name, value, threshold| {
                format!(
                    "Class '{}' has {} methods (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects classes with too many fields.
    ///
    /// Classes with many fields often have too many responsibilities and can be
    /// difficult to understand and maintain. Consider splitting the class or
    /// grouping related fields into separate objects.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing classes with too many fields,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    pub async fn detect_too_many_fields(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Class,
            config.max_fields,
            CodeSmellType::TooManyFields,
            "Group related fields into separate objects or split the class",
            |symbol_info| symbol_info.symbol.field_count(),
            |name, value, threshold| {
                format!(
                    "Class '{}' has {} fields (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects functions with high cyclomatic complexity.
    ///
    /// Cyclomatic complexity measures the number of independent paths through code.
    /// High complexity indicates functions that are difficult to test and maintain.
    /// Complex functions should be broken down into smaller, simpler functions.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing functions with high complexity,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    ///
    /// # Cyclomatic Complexity Reference
    /// - 1-4: Simple, easy to test
    /// - 5-7: Moderate, reasonably testable
    /// - 8-10: Complex, difficult to test
    /// - 11+: Very complex, should be refactored
    pub async fn detect_complex_conditionals(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Function,
            config.max_complexity,
            CodeSmellType::ComplexConditional,
            "Break down into smaller functions with single responsibilities",
            |symbol_info| symbol_info.symbol.complexity(),
            |name, value, threshold| {
                format!(
                    "Function '{}' has cyclomatic complexity of {} (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects code with excessive nesting depth.
    ///
    /// Deep nesting (nested if/for/while blocks) makes code harder to read and
    /// understand. Use early returns, extract functions, or flatten logic to
    /// reduce nesting.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing functions with deep nesting,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    ///
    /// # Nesting Depth Guidelines
    /// - 1-2: Ideal, easy to follow
    /// - 3-4: Acceptable, still readable
    /// - 5+: Problematic, hard to understand
    pub async fn detect_deep_nesting(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Function,
            config.max_nesting,
            CodeSmellType::DeepNesting,
            "Use early returns or extract nested logic into separate functions",
            |symbol_info| symbol_info.symbol.max_nesting_depth(),
            |name, value, threshold| {
                format!(
                    "Function '{}' has nesting depth of {} (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Detects functions with multiple return statements.
    ///
    /// While early returns can improve readability for guard clauses, excessive
    /// return statements create multiple exit points that make functions harder
    /// to reason about and debug. Consider refactoring to use a single return
    /// point or fewer returns.
    ///
    /// # Parameters
    /// - `graph`: The module graph containing all analyzed symbols
    /// - `config`: Configuration specifying quality thresholds
    ///
    /// # Returns
    /// A vector of findings representing functions with too many returns,
    /// or an error if symbol access fails.
    ///
    /// # Errors
    /// - `QualityAnalysisError::SymbolAccess`: If retrieving symbols fails
    /// - `QualityAnalysisError::ModuleAccess`: If retrieving a module fails
    /// - `QualityAnalysisError::ModuleNotFound`: If a referenced module doesn't exist
    ///
    /// # Return Statement Guidelines
    /// - 1: Ideal (single exit point)
    /// - 2-3: Acceptable (guard clauses + main logic)
    /// - 4+: Too many (complex control flow)
    pub async fn detect_multiple_returns(
        graph: &ModuleGraph,
        config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        Self::detect_metric_violation(
            graph,
            config,
            SymbolKind::Function,
            config.max_return_count,
            CodeSmellType::MultipleReturns,
            "Refactor to use single return point or reduce control flow complexity",
            |symbol_info| symbol_info.symbol.return_count(),
            |name, value, threshold| {
                format!(
                    "Function '{}' has {} return statements (recommended: {})",
                    name, value, threshold
                )
            },
        )
        .await
    }

    /// Generic detector for metric-based code quality violations.
    ///
    /// This helper method extracts the common pattern used across all metric-based
    /// detectors (long functions, too many parameters, large classes). It:
    /// 1. Gets all symbols from the module graph
    /// 2. Filters by the specified symbol kind
    /// 3. Gets the module for each symbol
    /// 4. Checks if the path is virtual (skips if so)
    /// 5. Extracts the metric value using the provided extractor
    /// 6. Compares against the threshold
    /// 7. Creates a finding if threshold is exceeded
    ///
    /// # Type Parameters
    /// - `F`: Function to extract the metric value from a symbol
    /// - `M`: Function to format the violation message
    ///
    /// # Parameters
    /// - `graph`: The module graph to analyze
    /// - `_config`: Configuration (parameter kept for future extensibility)
    /// - `kind`: The symbol kind to filter by (Function, Class, etc.)
    /// - `threshold`: The maximum acceptable value for the metric
    /// - `smell_type`: The type of code smell to report
    /// - `recommendation`: Suggested fix for the code smell
    /// - `metric_extractor`: Function to extract the metric from a symbol
    /// - `message_formatter`: Function to format the violation message
    ///
    /// # Design Rationale
    /// This generic method eliminates code duplication across the three detector
    /// methods while maintaining type safety and clarity. The use of closures
    /// allows each caller to specify how to extract metrics and format messages
    /// without duplicating the common filtering and error handling logic.
    #[allow(clippy::too_many_arguments)]
    async fn detect_metric_violation<F, M>(
        graph: &ModuleGraph,
        _config: &CodeQualityConfig,
        kind: SymbolKind,
        threshold: usize,
        smell_type: CodeSmellType,
        recommendation: &str,
        metric_extractor: F,
        message_formatter: M,
    ) -> Result<Vec<Finding>, QualityAnalysisError>
    where
        F: Fn(&UnusedSymbol) -> Option<usize>,
        M: Fn(&str, usize, usize) -> String,
    {
        let mut findings = Vec::new();

        // Get all symbols (not just unused ones) for code quality analysis
        let all_symbols = graph
            .all_symbols()
            .await
            .map_err(|e| QualityAnalysisError::SymbolAccess(e.to_string()))?;

        for symbol_info in all_symbols {
            // Only check symbols of the specified kind
            if !matches!(symbol_info.symbol.kind, ref k if *k == kind) {
                continue;
            }

            // Get the module containing this symbol
            let module = graph
                .module(&symbol_info.module_id)
                .await
                .map_err(|e| QualityAnalysisError::ModuleAccess {
                    module_id: format!("{:?}", symbol_info.module_id),
                    source: Box::new(std::io::Error::other(e.to_string())),
                })?
                .ok_or_else(|| {
                    QualityAnalysisError::ModuleNotFound(format!("{:?}", symbol_info.module_id))
                })?;

            // Skip virtual paths (bundler-generated synthetic modules)
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            // Extract the metric value and check against threshold
            if let Some(value) = metric_extractor(&symbol_info) {
                if value > threshold {
                    let message = message_formatter(&symbol_info.symbol.name, value, threshold);

                    findings.push(Self::create_smell_finding(
                        smell_type.clone(),
                        module.path.clone(),
                        Some(symbol_info.symbol.name.clone()),
                        Some(symbol_info.symbol.declaration_span.line),
                        Some(symbol_info.symbol.declaration_span.column),
                        SmellSeverity::Warning,
                        message,
                        Some(recommendation.to_string()),
                        Some(value),
                        Some(threshold),
                    ));
                }
            }
        }

        Ok(findings)
    }

    /// Detects magic numbers (hard-coded numeric literals).
    ///
    /// **NOTE**: This is a Tier 2 detector requiring AST traversal.
    /// Currently unimplemented and returns an empty vector. This placeholder
    /// exists for future implementation.
    ///
    /// Magic numbers are numeric literals that appear in code without explanation.
    /// They should typically be replaced with named constants for clarity.
    ///
    /// # Future Implementation
    /// Would require AST traversal to find all numeric literals and filter out
    /// acceptable cases like 0, 1, array indices, etc.
    pub async fn detect_magic_numbers(
        _graph: &ModuleGraph,
        _config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        // Would require AST traversal to find numeric literals
        // For MVP, placeholder - can be implemented later
        Ok(Vec::new())
    }

    /// Detects message chains (excessive method/property chaining).
    ///
    /// **NOTE**: This is a Tier 2 detector requiring AST traversal.
    /// Currently unimplemented and returns an empty vector. This placeholder
    /// exists for future implementation.
    ///
    /// Message chains like `obj.prop1.prop2.method1().prop3.method2()` violate
    /// the Law of Demeter and create tight coupling between objects.
    ///
    /// # Future Implementation
    /// Would require AST traversal to find member expression chains exceeding
    /// the configured threshold (typically 3-4 levels).
    pub async fn detect_message_chains(
        _graph: &ModuleGraph,
        _config: &CodeQualityConfig,
    ) -> Result<Vec<Finding>, QualityAnalysisError> {
        // Would require AST traversal to find member expression chains
        // For MVP, placeholder - can be implemented later
        Ok(Vec::new())
    }

    /// Creates a code smell finding with the specified parameters.
    ///
    /// This is a helper method that constructs the `Finding::CodeSmell` variant
    /// with all necessary metadata.
    ///
    /// # Parameters
    /// - `smell_type`: The type of code smell detected
    /// - `location`: File path where the smell was found
    /// - `symbol_name`: Name of the symbol (function, class, etc.)
    /// - `line`: Line number in the source file
    /// - `column`: Column number in the source file
    /// - `severity`: Severity level of the smell (typically Warning)
    /// - `message`: Human-readable description of the issue
    /// - `recommendation`: Suggested fix or best practice
    /// - `current_value`: The measured metric value (e.g., actual line count)
    /// - `recommended_threshold`: The configured threshold that was exceeded
    #[allow(clippy::too_many_arguments)]
    fn create_smell_finding(
        smell_type: CodeSmellType,
        location: PathBuf,
        symbol_name: Option<String>,
        line: Option<u32>,
        column: Option<u32>,
        severity: SmellSeverity,
        message: String,
        recommendation: Option<String>,
        current_value: Option<usize>,
        recommended_threshold: Option<usize>,
    ) -> Finding {
        Finding::CodeSmell {
            smell_type,
            location,
            symbol_name,
            line,
            column,
            severity,
            details: CodeSmellDetails {
                message,
                recommendation,
                current_value,
                recommended_threshold,
                metadata: HashMap::new(),
            },
        }
    }

    /// Checks if a path represents a virtual (bundler-generated) module.
    ///
    /// Virtual paths are synthetic modules created by bundlers (e.g., Rolldown)
    /// and should be filtered out from quality analysis since they don't
    /// represent actual source code.
    ///
    /// # Parameters
    /// - `path`: The file path to check
    ///
    /// # Returns
    /// `true` if the path starts with the virtual path prefix, `false` otherwise
    ///
    /// # Example
    /// ```
    /// # use std::path::PathBuf;
    /// # use danny_backend_js::analyzers::QualityAnalyzer;
    /// let virtual_path = PathBuf::from("virtual:rolldown-plugin-node-modules");
    /// // assert!(QualityAnalyzer::is_virtual_path(&virtual_path)); // private method
    /// ```
    fn is_virtual_path(path: &Path) -> bool {
        path.to_string_lossy().starts_with(VIRTUAL_PATH_PREFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the virtual path detection correctly identifies virtual paths.
    ///
    /// Virtual paths are used by bundlers to represent synthetic modules and
    /// should be filtered from analysis results.
    #[test]
    fn test_is_virtual_path() {
        // Virtual paths should be detected
        let virtual_path = PathBuf::from("virtual:rolldown-plugin-node-modules");
        assert!(QualityAnalyzer::is_virtual_path(&virtual_path));

        let virtual_path2 = PathBuf::from("virtual:anything");
        assert!(QualityAnalyzer::is_virtual_path(&virtual_path2));

        // Real paths should not be detected as virtual
        let real_path = PathBuf::from("src/components/Button.tsx");
        assert!(!QualityAnalyzer::is_virtual_path(&real_path));

        let absolute_path = PathBuf::from("/Users/test/project/index.ts");
        assert!(!QualityAnalyzer::is_virtual_path(&absolute_path));

        // Edge case: empty path
        let empty_path = PathBuf::new();
        assert!(!QualityAnalyzer::is_virtual_path(&empty_path));
    }

    /// Test error type formatting to ensure user-friendly messages.
    #[test]
    fn test_error_formatting() {
        let symbol_error =
            QualityAnalysisError::SymbolAccess("Database connection lost".to_string());
        let error_msg = format!("{}", symbol_error);
        assert_eq!(error_msg, "Failed to get symbols: Database connection lost");

        let module_error = QualityAnalysisError::ModuleAccess {
            module_id: "module_123".to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )),
        };
        let error_msg = format!("{}", module_error);
        assert!(error_msg.starts_with("Failed to get module module_123:"));
        assert!(error_msg.contains("File not found"));

        let not_found_error = QualityAnalysisError::ModuleNotFound("module_456".to_string());
        let error_msg = format!("{}", not_found_error);
        assert_eq!(error_msg, "Module not found: module_456");
    }

    /// Test that the create_smell_finding helper builds the correct Finding variant.
    #[test]
    fn test_create_smell_finding() {
        let finding = QualityAnalyzer::create_smell_finding(
            CodeSmellType::LongFunction,
            PathBuf::from("src/utils.ts"),
            Some("processData".to_string()),
            Some(42),
            Some(10),
            SmellSeverity::Warning,
            "Function is too long".to_string(),
            Some("Break into smaller functions".to_string()),
            Some(150),
            Some(50),
        );

        match finding {
            Finding::CodeSmell {
                smell_type,
                location,
                symbol_name,
                line,
                column,
                severity,
                details,
            } => {
                assert!(matches!(smell_type, CodeSmellType::LongFunction));
                assert_eq!(location, PathBuf::from("src/utils.ts"));
                assert_eq!(symbol_name, Some("processData".to_string()));
                assert_eq!(line, Some(42));
                assert_eq!(column, Some(10));
                assert!(matches!(severity, SmellSeverity::Warning));
                assert_eq!(details.message, "Function is too long");
                assert_eq!(
                    details.recommendation,
                    Some("Break into smaller functions".to_string())
                );
                assert_eq!(details.current_value, Some(150));
                assert_eq!(details.recommended_threshold, Some(50));
            }
            _ => panic!("Expected CodeSmell finding"),
        }
    }

    // Note: Integration tests for detect_* methods require a mock ModuleGraph.
    // These should be placed in the integration tests directory with proper
    // test fixtures. The tests above cover unit-testable components.
}
