//! JavaScript backend implementation using Fob.

use danny_core::types::SafetyAssessment;
use danny_core::{AnalysisOptions, AnalysisResult, Finding, LanguageBackend, Result};
use danny_fs::{FileSystem, NativeFileSystem};
use std::sync::Arc;

use crate::analyzers::{
    BundleSizeAnalyzer, ClassMemberAnalyzer, DependencyChainAnalyzer, DynamicImportAnalyzer,
    EnumMemberAnalyzer, NpmDependencyAnalyzer, QualityAnalyzer, SideEffectAnalyzer,
    TypeOnlyAnalyzer, UnusedExport as AnalyzerUnusedExport,
};
use danny_core::circular_deps::CircularDependencyDetector;
use danny_core::{AnalysisError, Dependency, ErrorSeverity, Statistics};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use crate::toml_config::DannyConfig;
use std::path::Path;

/// JavaScript/TypeScript analysis backend using Fob.
///
/// This backend delegates module resolution and dependency analysis to Fob,
/// converting Fob's graph structure into Danny's common [`Finding`] types.
///
/// # Examples
///
/// ```no_run
/// use danny_backend_js::JsBackend;
/// use danny_core::{AnalysisOptions, LanguageBackend};
/// use std::path::PathBuf;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = JsBackend::new()?;
///
/// let options = AnalysisOptions {
///     entry_points: vec![PathBuf::from("src/index.ts")],
///     project_root: PathBuf::from("."),
///     ..Default::default()
/// };
///
/// let result = backend.analyze(options)?;
/// println!("Analyzed {} modules", result.statistics.total_modules);
/// # Ok(())
/// # }
/// ```
pub struct JsBackend<F: FileSystem = NativeFileSystem> {
    runtime: tokio::runtime::Runtime,
    #[allow(dead_code)]
    // Used when created via with_filesystem(), otherwise filesystem is created per-analysis
    fs: Arc<F>,
}

impl<F: FileSystem> std::fmt::Debug for JsBackend<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsBackend")
            .field("runtime", &"Runtime")
            .field("fs", &"FileSystem")
            .finish()
    }
}

impl JsBackend {
    /// Creates a new JavaScript backend with default filesystem.
    ///
    /// The filesystem will be created dynamically based on the project_root
    /// in AnalysisOptions when analyze() is called.
    ///
    /// # Errors
    ///
    /// Returns an error if the Tokio runtime cannot be initialized.
    pub fn new() -> Result<Self> {
        // Create a temporary filesystem - will be replaced per-analysis
        // This is a workaround until we can make FileSystem creation lazy
        let temp_fs =
            Arc::new(
                NativeFileSystem::new(".").map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to create filesystem: {}", e),
                })?,
            );
        Self::with_filesystem(temp_fs)
    }

    /// Creates a new JavaScript backend with a custom filesystem.
    ///
    /// This is useful for WASM builds where you need to provide an in-memory filesystem.
    ///
    /// # Errors
    ///
    /// Returns an error if the Tokio runtime cannot be initialized.
    pub fn with_filesystem<F: FileSystem>(fs: Arc<F>) -> Result<JsBackend<F>> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| danny_core::Error::Backend {
            backend: "JavaScript".to_string(),
            message: format!("Failed to create Tokio runtime: {}", e),
        })?;

        Ok(JsBackend { runtime, fs })
    }
}

impl<F: FileSystem> JsBackend<F> {
    /// Loads Danny configuration from TOML file.
    ///
    /// If no config file exists, returns default configuration.
    async fn load_config<FS: FileSystem>(
        &self,
        options: &AnalysisOptions,
        fs: &Arc<FS>,
    ) -> Result<DannyConfig> {
        let config_path = if let Some(path) = &options.config_path {
            path.clone()
        } else {
            options.project_root.join(".danny.toml")
        };

        if !fs.exists(&config_path).await? {
            // Return default config if no file exists
            return Ok(DannyConfig::default());
        }

        let content = fs.read_to_string(&config_path).await?;

        toml::from_str(&content).map_err(|e| danny_core::Error::TomlError {
            file: config_path,
            source: e,
        })
    }

    /// Performs analysis using Fob's async API with a provided filesystem.
    ///
    /// This is the internal implementation that runs Fob's analysis.
    async fn analyze_async_with_fs<FS: FileSystem>(
        &self,
        options: AnalysisOptions,
        fs: Arc<FS>,
    ) -> Result<AnalysisResult> {
        use fob::analysis;

        let start = Instant::now();

        // Create Fob analysis options with Danny's TOML-based framework rules
        let framework_rules =
            danny_rule_engine::load_built_in_rules().map_err(|err| danny_core::Error::Backend {
                backend: "JavaScript".to_string(),
                message: format!("Failed to load TOML framework rules: {err}"),
            })?;

        let fob_options = fob::analysis::AnalyzeOptions {
            framework_rules,             // Use TOML-based rules from danny-rule-engine
            compute_usage_counts: false, // Default to false for performance
        };

        // Run TWO async tasks in parallel:
        // 1. Fob analysis (CPU-bound: parsing + bundling)
        // 2. File discovery (I/O-bound: walking directories)
        let fob_future = analysis::analyze_with_options(&options.entry_points, fob_options);
        let discovery_config = crate::file_discovery::DiscoveryConfig::default();
        let fs_clone = Arc::clone(&fs);
        let discovery_future =
            crate::file_discovery::discover_source_files(&options, &discovery_config, fs_clone);

        // Wait for both to complete
        let (fob_result, discovered_files) = tokio::try_join!(
            async {
                fob_future.await.map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Fob analysis failed: {}", e),
                })
            },
            discovery_future
        )?;

        // Convert Fob's graph to Danny findings
        let mut findings = self.convert_graph_to_findings(&fob_result.graph).await?;

        // Find unreachable files by comparing discovered files with module graph
        let unreachable_findings = crate::file_discovery::find_unreachable_files(
            discovered_files,
            &fob_result.graph,
            &options.entry_points,
        )
        .await?;

        findings.extend(unreachable_findings);

        // NEW: Optionally collect unused symbols
        let analyze_symbols = options
            .backend_options
            .get("symbols")
            .or_else(|| options.backend_options.get("detect_unused_symbols")) // Backward compatibility
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let symbol_stats = if analyze_symbols {
            let unused_symbols = fob_result.graph.unused_symbols().await.map_err(|e| {
                danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get unused symbols: {}", e),
                }
            })?;

            for unused in unused_symbols {
                let module = fob_result
                    .graph
                    .module(&unused.module_id)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get module: {}", e),
                    })?
                    .expect("Module must exist for unused symbol");

                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                // Filter out underscore-prefixed symbols (intentionally unused)
                if unused.symbol.name.starts_with('_') {
                    continue;
                }

                findings.push(Finding::UnusedSymbol {
                    module: module.path.clone(),
                    symbol_name: unused.symbol.name.clone(),
                    kind: Self::convert_symbol_kind(&unused.symbol.kind),
                    span: Self::convert_symbol_span(&unused.symbol.declaration_span, &module.path),
                    explanation: None,
                });
            }

            Some(Self::convert_symbol_stats(&fob_result.symbol_stats))
        } else {
            None
        };

        // NEW: Optionally detect code quality issues (code smells)
        let detect_quality = options
            .backend_options
            .get("quality")
            .or_else(|| options.backend_options.get("detect_quality")) // Backward compatibility
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut code_smell_findings = Vec::new();
        let mut code_smell_stats = None;

        if detect_quality {
            // Load config to get quality thresholds
            let config = self.load_config(&options, &fs).await?;
            let quality_config = &config.quality;

            // Validate configuration before running analysis
            quality_config
                .validate()
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Invalid quality configuration: {}", e),
                })?;

            // Run quality analyzers with proper error conversion
            let long_functions =
                QualityAnalyzer::detect_long_functions(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Long function detection failed: {}", e),
                    })?;

            let too_many_params =
                QualityAnalyzer::detect_too_many_parameters(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Parameter count detection failed: {}", e),
                    })?;

            let large_classes =
                QualityAnalyzer::detect_large_classes(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Large class detection failed: {}", e),
                    })?;

            let too_many_methods =
                QualityAnalyzer::detect_too_many_methods(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Too many methods detection failed: {}", e),
                    })?;

            let too_many_fields =
                QualityAnalyzer::detect_too_many_fields(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Too many fields detection failed: {}", e),
                    })?;

            let complex_conditionals =
                QualityAnalyzer::detect_complex_conditionals(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Complex conditional detection failed: {}", e),
                    })?;

            let deep_nesting =
                QualityAnalyzer::detect_deep_nesting(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Deep nesting detection failed: {}", e),
                    })?;

            let multiple_returns =
                QualityAnalyzer::detect_multiple_returns(&fob_result.graph, quality_config)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Multiple returns detection failed: {}", e),
                    })?;

            code_smell_findings.extend(long_functions);
            code_smell_findings.extend(too_many_params);
            code_smell_findings.extend(large_classes);
            code_smell_findings.extend(too_many_methods);
            code_smell_findings.extend(too_many_fields);
            code_smell_findings.extend(complex_conditionals);
            code_smell_findings.extend(deep_nesting);
            code_smell_findings.extend(multiple_returns);

            // Build code smell statistics
            use danny_core::types::{CodeSmellStats, CodeSmellType, SmellSeverity};
            use std::collections::HashMap;

            let mut by_type: HashMap<CodeSmellType, usize> = HashMap::new();
            let mut by_severity: HashMap<SmellSeverity, usize> = HashMap::new();

            for finding in &code_smell_findings {
                if let Finding::CodeSmell {
                    smell_type,
                    severity,
                    ..
                } = finding
                {
                    *by_type.entry(smell_type.clone()).or_insert(0) += 1;
                    *by_severity.entry(*severity).or_insert(0) += 1;
                }
            }

            code_smell_stats = Some(CodeSmellStats {
                total_smells: code_smell_findings.len(),
                by_type: by_type.into_iter().collect(),
                by_severity: by_severity.into_iter().collect(),
            });

            findings.extend(code_smell_findings);
        }

        // Collect statistics
        let modules = fob_result
            .graph
            .modules()
            .await
            .map_err(|e| danny_core::Error::Backend {
                backend: "JavaScript".to_string(),
                message: format!("Failed to get modules: {}", e),
            })?;
        let unused_exports =
            fob_result
                .graph
                .unused_exports()
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get unused exports: {}", e),
                })?;
        let unreachable_modules = fob_result.graph.unreachable_modules().await.map_err(|e| {
            danny_core::Error::Backend {
                backend: "JavaScript".to_string(),
                message: format!("Failed to get unreachable modules: {}", e),
            }
        })?;
        let framework_exports = fob_result
            .graph
            .framework_used_exports()
            .await
            .map_err(|e| danny_core::Error::Backend {
                backend: "JavaScript".to_string(),
                message: format!("Failed to get framework exports: {}", e),
            })?;

        // Phase 1: Collect dynamic import targets for side effect analysis
        let dynamic_import_targets: HashSet<PathBuf> = modules
            .iter()
            .flat_map(|m| &m.imports)
            .filter(|imp| matches!(imp.kind, fob::graph::ImportKind::Dynamic))
            .filter_map(|imp| imp.resolved_to.as_ref())
            .filter_map(|id| modules.iter().find(|m| &m.id == id))
            .map(|m| m.path.clone())
            .collect();

        // Phase 1: Enrich unreachable modules with safety assessment
        let mut enriched_unreachable = Vec::new();
        for module in &unreachable_modules {
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            let safety = SideEffectAnalyzer::assess_safety(
                module.has_side_effects,
                module.is_entry,
                &module.path,
                &dynamic_import_targets,
            );

            let safe_to_delete = matches!(safety, SafetyAssessment::SafeToDelete);

            // Update the finding with enriched metadata
            if let Some(Finding::UnreachableModule { metadata, .. }) = findings.iter_mut().find(
                |f| matches!(f, Finding::UnreachableModule { path: p, .. } if *p == module.path),
            ) {
                metadata.safe_to_delete = safe_to_delete;
                metadata.safety_assessment = safety.clone();
            }

            enriched_unreachable.push((
                module.path.clone(),
                module.original_size,
                module.has_side_effects,
            ));
        }

        // Phase 1: Calculate bundle size impact
        let bundle_impact = if !enriched_unreachable.is_empty() {
            Some(BundleSizeAnalyzer::calculate_impact(enriched_unreachable))
        } else {
            None
        };

        // Phase 1: Extract dynamic imports
        let dynamic_imports_data: Vec<_> = modules
            .iter()
            .flat_map(|m| {
                m.imports.iter().filter_map(|imp| {
                    if matches!(imp.kind, fob::graph::ImportKind::Dynamic) {
                        imp.resolved_to.as_ref().and_then(|id| {
                            modules.iter().find(|m2| &m2.id == id).map(|target_module| {
                                (
                                    m.path.clone(),
                                    target_module.path.clone(),
                                    imp.source.clone(),
                                    imp.is_external(),
                                )
                            })
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        let dynamic_imports = DynamicImportAnalyzer::extract_dynamic_imports(dynamic_imports_data);
        for dyn_import in &dynamic_imports {
            findings.push(Finding::DynamicImport(dyn_import.clone()));
        }

        // Phase 1: Categorize unused exports by type-only vs runtime
        let mut analyzer_unused_exports: Vec<AnalyzerUnusedExport> = Vec::new();
        for unused in &unused_exports {
            let module = fob_result
                .graph
                .module(&unused.module_id)
                .await
                .ok()
                .flatten();

            if let Some(module) = module {
                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                analyzer_unused_exports.push(AnalyzerUnusedExport {
                    module: module.path.clone(),
                    name: unused.export.name.clone(),
                    is_type_only: unused.export.is_type_only,
                });
            }
        }

        let categorized_exports = TypeOnlyAnalyzer::categorize_exports(analyzer_unused_exports);
        let type_only_count = categorized_exports.type_only.len();

        // Phase 1: Detect circular dependencies
        let mut dependency_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for module in &modules {
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            let deps: Vec<PathBuf> = module
                .imports
                .iter()
                .filter_map(|imp| {
                    imp.resolved_to.as_ref().and_then(|id| {
                        modules
                            .iter()
                            .find(|m2| &m2.id == id)
                            .map(|m2| m2.path.clone())
                    })
                })
                .filter(|p| !Self::is_virtual_path(p))
                .collect();

            if !deps.is_empty() {
                dependency_graph.insert(module.path.clone(), deps);
            }
        }

        let mut circular_detector = CircularDependencyDetector::new(dependency_graph);
        let circular_deps = circular_detector.find_cycles().unwrap_or_default();
        let circular_deps_count = circular_deps.len();

        // Add circular dependency findings
        for circ_dep in &circular_deps {
            // Check if all modules in cycle are unreachable
            let all_unreachable = circ_dep
                .cycle
                .iter()
                .all(|path| unreachable_modules.iter().any(|m| m.path == *path));

            // Calculate total size
            let total_size: usize = circ_dep
                .cycle
                .iter()
                .filter_map(|path| {
                    modules
                        .iter()
                        .find(|m| m.path == *path)
                        .map(|m| m.original_size)
                })
                .sum();

            findings.push(Finding::CircularDependency(
                danny_core::types::CircularDependency {
                    cycle: circ_dep.cycle.clone(),
                    all_unreachable,
                    total_size,
                },
            ));
        }

        // Count unreachable files separately from unreachable modules
        let unreachable_files_count = findings
            .iter()
            .filter(|f| matches!(f, Finding::UnreachableFile { .. }))
            .count();

        // NEW: Class Member Analysis (opt-in via detect_class_members)
        let detect_class_members = options
            .backend_options
            .get("detect_class_members")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (private_class_findings, public_class_findings, class_member_stats) =
            if detect_class_members {
                let unused_private = fob_result
                    .graph
                    .unused_private_class_members()
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get unused private class members: {}", e),
                    })?;

                let unused_public = fob_result
                    .graph
                    .unused_public_class_members()
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get unused public class members: {}", e),
                    })?;

                let private_findings = ClassMemberAnalyzer::convert_private_members(
                    &fob_result.graph,
                    &unused_private,
                )
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to convert private class members: {}", e),
                })?;

                let public_findings =
                    ClassMemberAnalyzer::convert_public_members(&fob_result.graph, &unused_public)
                        .await
                        .map_err(|e| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to convert public class members: {}", e),
                        })?;

                let total_private: usize = unused_private.values().map(|v| v.len()).sum();
                let total_public = unused_public.len();
                let total_members = total_private + total_public;

                let stats = Some(ClassMemberAnalyzer::build_stats(
                    total_private,
                    total_public,
                    total_members,
                ));

                (private_findings, public_findings, stats)
            } else {
                (Vec::new(), Vec::new(), None)
            };

        findings.extend(private_class_findings);
        findings.extend(public_class_findings);

        // NEW: Enum Analysis (opt-in via detect_enum_members)
        let detect_enum_members = options
            .backend_options
            .get("detect_enum_members")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (enum_findings, enum_stats) = if detect_enum_members {
            let unused_enums = fob_result.graph.unused_enum_members().await.map_err(|e| {
                danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get unused enum members: {}", e),
                }
            })?;

            let enum_findings =
                EnumMemberAnalyzer::convert_unused_members(&fob_result.graph, &unused_enums)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to convert enum members: {}", e),
                    })?;

            let total_enums = unused_enums.len();
            let total_members: usize = unused_enums.values().map(|v| v.len()).sum();
            let unused_members = enum_findings.len();

            let stats = Some(EnumMemberAnalyzer::build_stats(
                total_enums,
                total_members,
                unused_members,
            ));

            (enum_findings, stats)
        } else {
            (Vec::new(), None)
        };

        findings.extend(enum_findings);

        // NEW: NPM Dependencies (opt-in via detect_npm_dependencies)
        let detect_npm_dependencies = options
            .backend_options
            .get("detect_npm_dependencies")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (npm_findings, dependency_coverage_stats) = if detect_npm_dependencies {
            let package_json_path = options.project_root.join("package.json");
            if fs.exists(&package_json_path).await? {
                // Parse package.json using serde_json since PackageJson implements Deserialize
                let package_json_content = fs.read_to_string(&package_json_path).await?;
                let package_json: fob::graph::PackageJson =
                    serde_json::from_str(&package_json_content).map_err(|e| {
                        danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to parse package.json: {}", e),
                        }
                    })?;

                let unused_deps = fob_result
                    .graph
                    .unused_npm_dependencies(
                        &package_json,
                        true,  // include_dev
                        false, // include_peer
                    )
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get unused npm dependencies: {}", e),
                    })?;

                let coverage = fob_result
                    .graph
                    .dependency_coverage(&package_json)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get dependency coverage: {}", e),
                    })?;

                let npm_findings = NpmDependencyAnalyzer::convert_unused_dependencies(&unused_deps);
                let coverage_stats = Some(NpmDependencyAnalyzer::convert_coverage_stats(&coverage));

                (npm_findings, coverage_stats)
            } else {
                (Vec::new(), None)
            }
        } else {
            (Vec::new(), None)
        };

        findings.extend(npm_findings);

        // NEW: Import Patterns (opt-in via detect_import_patterns)
        let detect_import_patterns = options
            .backend_options
            .get("detect_import_patterns")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (side_effect_findings, namespace_findings, type_only_import_findings) =
            if detect_import_patterns {
                let side_effects =
                    fob_result
                        .graph
                        .side_effect_only_imports()
                        .await
                        .map_err(|e| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to get side-effect-only imports: {}", e),
                        })?;

                let namespaces = fob_result.graph.namespace_imports().await.map_err(|e| {
                    danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get namespace imports: {}", e),
                    }
                })?;

                let type_only_imports =
                    fob_result.graph.type_only_imports().await.map_err(|e| {
                        danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to get type-only imports: {}", e),
                        }
                    })?;

                let mut side_effect_findings = Vec::new();
                let mut namespace_findings = Vec::new();
                let mut type_only_import_findings = Vec::new();

                for side_effect in &side_effects {
                    let module = fob_result
                        .graph
                        .module(&side_effect.importer)
                        .await
                        .map_err(|e| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to get module: {}", e),
                        })?
                        .ok_or_else(|| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Module not found: {:?}", side_effect.importer),
                        })?;

                    if Self::is_virtual_path(&module.path) {
                        continue;
                    }

                    let resolved_to = if let Some(id) = &side_effect.resolved_to {
                        fob_result
                            .graph
                            .module(id)
                            .await
                            .ok()
                            .flatten()
                            .map(|m| m.path)
                    } else {
                        None
                    };

                    side_effect_findings.push(Finding::SideEffectOnlyImport {
                        module: module.path.clone(),
                        source: side_effect.source.clone(),
                        resolved_to,
                        span: Self::convert_span(&side_effect.span),
                    });
                }

                for namespace in &namespaces {
                    let module = fob_result
                        .graph
                        .module(&namespace.importer)
                        .await
                        .map_err(|e| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to get module: {}", e),
                        })?
                        .ok_or_else(|| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Module not found: {:?}", namespace.importer),
                        })?;

                    if Self::is_virtual_path(&module.path) {
                        continue;
                    }

                    let resolved_to = if let Some(id) = &namespace.resolved_to {
                        fob_result
                            .graph
                            .module(id)
                            .await
                            .ok()
                            .flatten()
                            .map(|m| m.path)
                    } else {
                        None
                    };

                    namespace_findings.push(Finding::NamespaceImport {
                        module: module.path.clone(),
                        namespace_name: namespace.namespace_name.clone(),
                        source: namespace.source.clone(),
                        resolved_to,
                    });
                }

                for type_import in &type_only_imports {
                    let module = fob_result
                        .graph
                        .module(&type_import.importer)
                        .await
                        .map_err(|e| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Failed to get module: {}", e),
                        })?
                        .ok_or_else(|| danny_core::Error::Backend {
                            backend: "JavaScript".to_string(),
                            message: format!("Module not found: {:?}", type_import.importer),
                        })?;

                    if Self::is_virtual_path(&module.path) {
                        continue;
                    }

                    let specifiers: Vec<String> = type_import
                        .specifiers
                        .iter()
                        .map(|s| match s {
                            fob::graph::ImportSpecifier::Named(name) => name.clone(),
                            fob::graph::ImportSpecifier::Default => "default".to_string(),
                            fob::graph::ImportSpecifier::Namespace(name) => name.clone(),
                        })
                        .collect();

                    type_only_import_findings.push(Finding::TypeOnlyImport {
                        module: module.path.clone(),
                        source: type_import.source.clone(),
                        specifiers,
                        span: Self::convert_span(&type_import.span),
                    });
                }

                (
                    side_effect_findings,
                    namespace_findings,
                    type_only_import_findings,
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };

        findings.extend(side_effect_findings);
        findings.extend(namespace_findings);
        findings.extend(type_only_import_findings);

        // NEW: Dead Code Modules (opt-in via detect_dead_code_modules)
        let detect_dead_code_modules = options
            .backend_options
            .get("detect_dead_code_modules")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let dead_code_findings = if detect_dead_code_modules {
            let mut dead_code_findings = Vec::new();
            let all_modules =
                fob_result
                    .graph
                    .modules()
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to get modules: {}", e),
                    })?;

            for module in &all_modules {
                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                let is_dead = fob_result
                    .graph
                    .is_reachable_only_through_dead_code(&module.id)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to check dead code status: {}", e),
                    })?;

                if is_dead {
                    dead_code_findings.push(Finding::DeadCodeModule {
                        path: module.path.clone(),
                        size: module.original_size,
                    });
                }
            }

            dead_code_findings
        } else {
            Vec::new()
        };

        findings.extend(dead_code_findings);

        // NEW: Dependency Chain Analysis (opt-in via detect_dependency_chains)
        let detect_dependency_chains = options
            .backend_options
            .get("detect_dependency_chains")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if detect_dependency_chains {
            // Analyze dependency chains for modules with deep imports (depth > 5)
            // This helps identify overly nested module dependencies
            for module in &modules {
                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                // Get chains to this module
                let chains_result = fob_result.graph.dependency_chains_to(&module.id).await;

                if let Ok(chains) = chains_result {
                    // Only report chains that are deeper than 5 levels
                    // to avoid too many findings
                    let deep_chains: Vec<_> =
                        chains.into_iter().filter(|chain| chain.depth > 5).collect();

                    if !deep_chains.is_empty() {
                        match DependencyChainAnalyzer::convert_chains(
                            &fob_result.graph,
                            &deep_chains,
                        )
                        .await
                        {
                            Ok(chain_findings) => findings.extend(chain_findings),
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to convert dependency chains for {:?}: {}",
                                    module.path, e
                                );
                            }
                        }
                    }
                }
            }
        }

        let statistics = Statistics {
            total_modules: modules.len(),
            total_dependencies: modules.iter().map(|m| m.imports.len()).sum(),
            external_dependencies: fob_result.stats.external_dependency_count,
            frameworks_detected: self.detect_frameworks(&fob_result.graph).await,
            unused_exports_count: unused_exports.len(),
            unreachable_modules_count: unreachable_modules.len(),
            unreachable_files_count,
            framework_exports_count: framework_exports.len(),
            symbol_statistics: symbol_stats,
            bundle_size_impact: bundle_impact,
            dynamic_imports_count: dynamic_imports.len(),
            circular_dependencies_count: circular_deps_count,
            type_only_unused_exports_count: type_only_count,
            unused_private_class_members_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::UnusedPrivateClassMember { .. }))
                .count(),
            unused_public_class_members_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::UnusedPublicClassMember { .. }))
                .count(),
            unused_enum_members_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::UnusedEnumMember { .. }))
                .count(),
            unused_npm_dependencies_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::UnusedNpmDependency { .. }))
                .count(),
            side_effect_only_imports_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::SideEffectOnlyImport { .. }))
                .count(),
            namespace_imports_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::NamespaceImport { .. }))
                .count(),
            type_only_imports_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::TypeOnlyImport { .. }))
                .count(),
            dead_code_modules_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::DeadCodeModule { .. }))
                .count(),
            dependency_chains_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::DependencyChain { .. }))
                .count(),
            class_member_stats,
            enum_stats,
            dependency_coverage_stats,
            ignored_findings_count: 0, // CLI will update this during filtering
            ignored_findings_breakdown: None, // CLI will update this during filtering
            duration_ms: start.elapsed().as_millis() as u64,
            code_quality_stats: code_smell_stats,
            code_smells_count: findings
                .iter()
                .filter(|f| matches!(f, Finding::CodeSmell { .. }))
                .count(),
        };

        // Convert errors and warnings
        let errors: Vec<AnalysisError> = fob_result
            .errors
            .into_iter()
            .map(|msg| AnalysisError {
                file: PathBuf::from("unknown"), // Fob doesn't provide file paths in error messages
                message: msg,
                severity: ErrorSeverity::Error,
            })
            .chain(fob_result.warnings.into_iter().map(|msg| AnalysisError {
                file: PathBuf::from("unknown"),
                message: msg,
                severity: ErrorSeverity::Warning,
            }))
            .collect();

        Ok(AnalysisResult {
            findings,
            statistics,
            errors,
            ignored_findings: vec![], // CLI will populate this during filtering
        })
    }

    /// Converts Fob's module graph to Danny findings.
    async fn convert_graph_to_findings(
        &self,
        graph: &fob::graph::ModuleGraph,
    ) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Convert each module to a Module finding
        let modules = graph
            .modules()
            .await
            .map_err(|e| danny_core::Error::Backend {
                backend: "JavaScript".to_string(),
                message: format!("Failed to get modules: {}", e),
            })?;

        for module in modules {
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            let dependencies: Vec<Dependency> = module
                .imports
                .iter()
                .map(|import| Dependency {
                    specifier: import.source.clone(),
                    resolved: import
                        .resolved_to
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| import.source.clone()),
                    is_external: import.is_external(),
                    is_dynamic: matches!(import.kind, fob::graph::ImportKind::Dynamic),
                })
                .collect();

            findings.push(Finding::Module {
                path: module.path.clone(),
                dependencies: dependencies.clone(),
                metadata: HashMap::new(),
            });

            // Also add individual dependency findings
            for dep in dependencies {
                if Self::is_virtual_resolved(&dep.resolved) {
                    continue;
                }

                findings.push(Finding::Dependency {
                    from: module.path.clone(),
                    to: PathBuf::from(&dep.resolved),
                    specifier: dep.specifier.clone(),
                    is_external: dep.is_external,
                });
            }
        }

        // NEW: Detect unused exports
        let unused_exports =
            graph
                .unused_exports()
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get unused exports: {}", e),
                })?;

        for unused in unused_exports {
            let module = graph
                .module(&unused.module_id)
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get module: {}", e),
                })?
                .expect("Module must exist for unused export");

            if Self::is_virtual_path(&module.path) {
                continue;
            }

            findings.push(Finding::UnusedExport {
                module: module.path.clone(),
                export_name: unused.export.name.clone(),
                kind: Self::convert_export_kind(&unused.export.kind),
                span: Some(Self::convert_span(&unused.export.span)),
                is_type_only: unused.export.is_type_only,
                explanation: None,
            });
        }

        // NEW: Detect unreachable modules
        // Note: Safety assessment and bundle impact will be calculated in analyze_async
        // after we have all the data (dynamic imports, etc.)
        let unreachable_modules =
            graph
                .unreachable_modules()
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get unreachable modules: {}", e),
                })?;

        for module in unreachable_modules {
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            findings.push(Finding::UnreachableModule {
                path: module.path.clone(),
                size: module.original_size,
                metadata: danny_core::types::UnreachableModuleMetadata {
                    has_side_effects: module.has_side_effects,
                    size_bytes: module.original_size,
                    safe_to_delete: false, // Will be enriched in analyze_async
                    safety_assessment: SafetyAssessment::Unsafe("Pending analysis".to_string()), // Will be enriched in analyze_async
                },
            });
        }

        // NEW: Report framework-used exports (for transparency)
        let framework_exports =
            graph
                .framework_used_exports()
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get framework exports: {}", e),
                })?;

        for (module_id, export) in framework_exports {
            let module = graph
                .module(&module_id)
                .await
                .map_err(|e| danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to get module: {}", e),
                })?
                .expect("Module must exist for framework export");

            if Self::is_virtual_path(&module.path) {
                continue;
            }

            let framework = Self::infer_framework(&module, &export.name);

            findings.push(Finding::FrameworkExport {
                module: module.path.clone(),
                export_name: export.name.clone(),
                framework,
                rule: "Built-in".to_string(),
                explanation: None,
            });
        }

        Ok(findings)
    }

    fn is_virtual_path(path: &Path) -> bool {
        path.to_string_lossy().starts_with("virtual:")
    }

    fn is_virtual_resolved(resolved: &str) -> bool {
        resolved.starts_with("virtual:")
    }

    /// Converts Fob's ExportKind to Danny's ExportKind.
    fn convert_export_kind(kind: &fob::graph::ExportKind) -> danny_core::ExportKind {
        use danny_core::ExportKind;
        match kind {
            fob::graph::ExportKind::Named => ExportKind::Named,
            fob::graph::ExportKind::Default => ExportKind::Default,
            fob::graph::ExportKind::ReExport => ExportKind::ReExport,
            fob::graph::ExportKind::StarReExport => ExportKind::StarReExport,
            fob::graph::ExportKind::TypeOnly => ExportKind::TypeOnly,
        }
    }

    /// Converts Fob's SourceSpan to Danny's SourceLocation.
    fn convert_span(span: &fob::graph::SourceSpan) -> danny_core::SourceLocation {
        use danny_core::SourceLocation;
        SourceLocation {
            file: span.file.clone(),
            start: span.start,
            end: span.end,
        }
    }

    /// Converts Fob's SymbolKind to Danny's SymbolKind.
    fn convert_symbol_kind(kind: &fob::graph::SymbolKind) -> danny_core::types::SymbolKind {
        use danny_core::types::SymbolKind;
        match kind {
            fob::graph::SymbolKind::Function => SymbolKind::Function,
            fob::graph::SymbolKind::Variable => SymbolKind::Variable,
            fob::graph::SymbolKind::Class => SymbolKind::Class,
            fob::graph::SymbolKind::Parameter => SymbolKind::Parameter,
            fob::graph::SymbolKind::TypeAlias => SymbolKind::TypeAlias,
            fob::graph::SymbolKind::Interface => SymbolKind::Interface,
            fob::graph::SymbolKind::Enum => SymbolKind::Enum,
            _ => SymbolKind::Variable, // Fallback for any other types
        }
    }

    /// Converts Fob's SymbolSpan to Danny's SymbolSpan (with file path from module).
    fn convert_symbol_span(
        span: &fob::graph::SymbolSpan,
        file: &std::path::Path,
    ) -> danny_core::types::SymbolSpan {
        danny_core::types::SymbolSpan {
            file: file.to_path_buf(),
            line: span.line,
            column: span.column,
            offset: span.offset,
        }
    }

    /// Converts Fob's SymbolStatistics to Danny's SymbolStats.
    fn convert_symbol_stats(
        stats: &fob::graph::SymbolStatistics,
    ) -> danny_core::types::SymbolStats {
        danny_core::types::SymbolStats {
            total_symbols: stats.total_symbols,
            unused_symbols: stats.unused_symbols,
            by_kind: stats
                .by_kind
                .iter()
                .map(|(kind, count)| (Self::convert_symbol_kind(kind), *count))
                .collect(),
        }
    }

    /// Infers framework from module imports and export patterns using TOML-based detection.
    fn infer_framework(module: &fob::graph::Module, _export_name: &str) -> String {
        // Load TOML-based detector
        let toml_files = danny_rule_engine::built_in::load_built_in_toml_files();
        let detector = match danny_rule_engine::FrameworkDetector::from_toml_files(toml_files) {
            Ok(d) => d,
            Err(_) => {
                // Fallback to "Unknown" if detection fails
                return "Unknown".to_string();
            }
        };

        // Extract imports
        let imports: Vec<String> = module
            .imports
            .iter()
            .map(|imp| imp.source.clone())
            .collect();

        // Detect from imports
        let results = detector.detect_from_imports(&imports);
        if let Some(result) = results.first() {
            return result.framework.clone();
        }

        // Detect from file path
        let path = std::path::Path::new(&module.path);
        let path_results = detector.detect_from_path(path);
        if let Some(result) = path_results.first() {
            return result.framework.clone();
        }

        "Unknown".to_string()
    }

    /// Detects frameworks from the graph using TOML-based detection.
    async fn detect_frameworks(&self, graph: &fob::graph::ModuleGraph) -> Vec<String> {
        // Load TOML-based detector
        let toml_files = danny_rule_engine::built_in::load_built_in_toml_files();
        let detector = match danny_rule_engine::FrameworkDetector::from_toml_files(toml_files) {
            Ok(d) => d,
            Err(_) => {
                // Return empty if detection fails
                return Vec::new();
            }
        };

        let modules = match graph.modules().await {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        // Collect all imports from all modules
        let mut all_imports = Vec::new();
        for module in &modules {
            for import in &module.imports {
                all_imports.push(import.source.clone());
            }
        }

        // Detect frameworks from imports
        let results = detector.detect_from_imports(&all_imports);

        // Also check file paths
        let mut path_results = Vec::new();
        for module in &modules {
            let path = std::path::Path::new(&module.path);
            let results = detector.detect_from_path(path);
            path_results.extend(results);
        }

        // Combine results, preferring import-based detection
        let mut detected_frameworks: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for result in results {
            detected_frameworks.insert(result.framework);
        }
        for result in path_results {
            detected_frameworks.insert(result.framework);
        }

        detected_frameworks.into_iter().collect()
    }
}

impl<F: FileSystem> LanguageBackend for JsBackend<F> {
    fn name(&self) -> &str {
        "JavaScript"
    }

    fn supported_extensions(&self) -> &[&str] {
        &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"]
    }

    fn analyze(&self, options: AnalysisOptions) -> Result<AnalysisResult> {
        // Run async analysis in the runtime
        self.runtime.block_on(async {
            // Create filesystem scoped to project root for this analysis
            let fs = Arc::new(NativeFileSystem::new(&options.project_root).map_err(|e| {
                danny_core::Error::Backend {
                    backend: "JavaScript".to_string(),
                    message: format!("Failed to create filesystem: {}", e),
                }
            })?);

            // Validate that entry points exist using FileSystem abstraction
            for entry in &options.entry_points {
                if !fs
                    .exists(entry)
                    .await
                    .map_err(|e| danny_core::Error::Backend {
                        backend: "JavaScript".to_string(),
                        message: format!("Failed to check if entry point exists: {}", e),
                    })?
                {
                    return Err(danny_core::Error::EntryPointNotFound {
                        path: entry.clone(),
                    });
                }
            }

            // Load configuration (for future use with custom framework rules)
            let _config = self.load_config(&options, &fs).await?;

            self.analyze_async_with_fs(options, fs).await
        })
    }

    fn validate(&self, options: &AnalysisOptions) -> Result<()> {
        // Validate entry point extensions (filesystem-independent check)
        for entry in &options.entry_points {
            let ext = entry
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            if !self.supported_extensions().contains(&ext.as_str()) {
                return Err(danny_core::Error::InvalidConfig {
                    message: format!("Entry point {:?} has unsupported extension: {}", entry, ext),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_backend_creation() {
        let backend = JsBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_backend_name() {
        let backend = JsBackend::new().unwrap();
        assert_eq!(backend.name(), "JavaScript");
    }

    #[test]
    fn test_supported_extensions() {
        let backend = JsBackend::new().unwrap();
        let extensions = backend.supported_extensions();
        assert!(extensions.contains(&".js"));
        assert!(extensions.contains(&".ts"));
        assert!(extensions.contains(&".jsx"));
        assert!(extensions.contains(&".tsx"));
    }

    #[test]
    fn test_validate_missing_entry_point() {
        let backend = JsBackend::new().unwrap();
        let options = AnalysisOptions {
            // Use a relative path within project root that doesn't exist
            entry_points: vec![PathBuf::from("nonexistent_file_xyz123.ts")],
            project_root: PathBuf::from("."),
            ..Default::default()
        };

        // analyze() now does the filesystem check using FileSystem abstraction
        let result = backend.analyze(options);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            danny_core::Error::EntryPointNotFound { .. }
        ));
    }
}
