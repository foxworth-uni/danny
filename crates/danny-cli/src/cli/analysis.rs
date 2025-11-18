//! Analysis orchestration logic with category capabilities.

use anyhow::{Context, Result};
use danny_backend_js::JsBackend;
use danny_core::{AnalysisOptions, BackendRegistry, Category};
use crate::cli::category::{CategoryValidator, CategoryValidation};
use crate::display::CapabilityDisplay;
use crate::entry_points::EntryPointDetector;
use crate::formatters;
use std::path::PathBuf;

/// Output format for results.
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

/// Options for running analysis.
pub struct AnalysisRunOptions {
    pub paths: Vec<PathBuf>,
    pub categories: Vec<Category>,
    pub list_categories: bool,
    pub force_package_mode: bool,
    pub yes: bool,
    pub config: Option<PathBuf>,
    pub follow_external: bool,
    pub max_depth: Option<usize>,
    pub no_ignore: bool,
    pub no_gitignore: bool,
    pub ignore_patterns: Vec<String>,
    pub verbose: u8,
    pub json: bool,
    pub format: OutputFormat,
}

/// Runs the analysis with the given options.
pub fn run_analysis(options: &AnalysisRunOptions) -> Result<()> {
    let working_dir = std::env::current_dir()
        .context("Failed to get current working directory")?;

    // Step 1: Detect analysis target (Package or Files mode)
    let detector = EntryPointDetector::new(working_dir);
    let target = detector
        .detect_target(&options.paths, options.force_package_mode)
        .context("Failed to detect analysis target")?;

    // Step 2: Handle --list-categories flag
    if options.list_categories {
        let display = CapabilityDisplay::new(target, options.yes);
        display.display_list_categories();
        return Ok(());
    }

    // Step 3: Validate requested categories
    let validator = CategoryValidator::new(&target);
    
    // Show capabilities in verbose mode
    if options.verbose > 0 {
        let caps = validator.capabilities();
        println!("Analysis capabilities: {} available, {} unavailable",
                 caps.available_categories().len(),
                 caps.unavailable_categories().len());
    }
    
    let validation = validator.validate(&options.categories);

    // Step 4: Display warnings and get confirmation if needed
    let display = CapabilityDisplay::new(target.clone(), options.yes);
    
    // Use the convenience method to check if confirmation is needed
    if validation.requires_confirmation() && !options.yes {
        // Show warning for partial availability
        if let CategoryValidation::PartiallyAvailable { available, unavailable } = &validation {
            display.show_partial_warning(available, unavailable);
            if !display.confirm_continue()? {
                eprintln!("Cancelled");
                std::process::exit(1);
            }
        }
    }
    
    // Use the convenience method to extract categories
    let categories = match validation {
        CategoryValidation::UseDefaults { categories } => {
            if !options.yes {
                display.show_using_defaults(&categories);
            }
            categories
        }

        CategoryValidation::AllAvailable { categories } => categories,

        CategoryValidation::PartiallyAvailable { available, .. } => {
            // Already confirmed above if needed
            available
        }

        CategoryValidation::NoneAvailable { requested, unavailable } => {
            display.show_none_available(&requested, &unavailable);
            std::process::exit(1);
        }
    };

    // Step 5: Get project root and entry points
    let project_root = target.root_dir();
    let entry_points = match &target {
        danny_config::AnalysisTarget::Package(pkg) => pkg.entry_points.clone(),
        danny_config::AnalysisTarget::Files(files) => files.files.clone(),
    };

    if entry_points.is_empty() {
        eprintln!("No entry points found. Please specify paths or ensure package.json exists.");
        std::process::exit(1);
    }

    // Step 6: Build ignore patterns
    let (ignore_set, pattern_infos) = if options.no_ignore {
        crate::ignore::IgnorePatternBuilder::new()
            .no_defaults()
            .build_with_metadata()
            .context("Failed to build ignore patterns")?
    } else {
        let mut builder = crate::ignore::IgnorePatternBuilder::new();

        if !options.ignore_patterns.is_empty() {
            builder = builder
                .add_patterns(&options.ignore_patterns)
                .context("Invalid ignore pattern")?;
        }

        if !options.no_gitignore {
            match crate::ignore::load_gitignore_patterns(project_root) {
                Ok(gitignore_patterns) => {
                    let patterns: Vec<_> = gitignore_patterns
                        .into_iter()
                        .filter(|p| !p.is_empty())
                        .collect();

                    if !patterns.is_empty() {
                        builder = builder
                            .add_patterns(patterns)
                            .context("Invalid .gitignore pattern")?;
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse .gitignore: {}", e);
                }
            }
        }

        builder
            .build_with_metadata()
            .context("Failed to build ignore patterns")?
    };

    // Step 7: Create backend registry
    let mut registry = BackendRegistry::new();
    let js_backend = JsBackend::new().context("Failed to create JavaScript backend")?;
    registry.register(Box::new(js_backend));

    // Step 8: Detect backend from first entry point
    let first_entry = &entry_points[0];
    let extension = first_entry
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let backend = registry
        .find_by_extension(&extension)
        .ok_or_else(|| anyhow::anyhow!("No backend found for extension: {}", extension))?;

    // Step 9: Build analysis options
    let mut backend_options = std::collections::HashMap::new();
    
    // Enable analysis based on requested categories
    if categories.contains(&Category::Symbols) {
        backend_options.insert(
            "symbols".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    if categories.contains(&Category::Quality) {
        backend_options.insert(
            "quality".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    if categories.contains(&Category::Dependencies) {
        backend_options.insert(
            "detect_npm_dependencies".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    if categories.contains(&Category::Imports) {
        backend_options.insert(
            "detect_import_patterns".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    // Dead code analysis (Files, Exports, Types, Circular, Framework) is always enabled if requested

    let analysis_options = AnalysisOptions {
        entry_points,
        project_root: project_root.clone(),
        follow_external: options.follow_external,
        max_depth: options.max_depth,
        config_path: options.config.clone(),
        backend_options,
    };

    // Step 10: Validate before analysis
    backend.validate(&analysis_options).context("Validation failed")?;

    // Step 11: Determine output format
    let output_format = if options.json {
        OutputFormat::Json
    } else {
        options.format
    };

    // Step 12: Perform analysis
    match output_format {
        OutputFormat::Json => {
            eprintln!("Analyzing {} entry points...", analysis_options.entry_points.len());
        }
        OutputFormat::Human => {
            if options.verbose > 0 {
                println!("Running analysis...");
                println!("Mode: {:?}", target.mode());
                println!("Categories: {:?}", categories);
            }
        }
    }

    let mut result = backend.analyze(analysis_options).context("Analysis failed")?;

    // Step 13: Filter findings based on ignore patterns
    if !options.no_ignore {
        let filter_result =
            crate::cli::filtering::filter_findings_with_tracking(
                result.findings,
                &ignore_set,
                &pattern_infos,
            );

        result.findings = filter_result.kept;
        result.ignored_findings = filter_result.ignored;

        let ignore_breakdown =
            crate::cli::filtering::calculate_ignore_statistics(&result.ignored_findings);
        result.statistics.ignored_findings_count = result.ignored_findings.len();
        result.statistics.ignored_findings_breakdown = Some(ignore_breakdown);

        crate::cli::filtering::recalculate_statistics(&mut result);
    }

    // Step 14: Filter findings by requested categories
    result.findings.retain(|finding| {
        categories.contains(&finding.category())
    });

    // Step 15: Output results
    use formatters::Formatter;
    match output_format {
        OutputFormat::Json => {
            let formatter = formatters::JsonFormatter;
            formatter.format(&result);
        }
        OutputFormat::Human => {
            let formatter = formatters::HumanFormatter;
            formatter.format(&result);
        }
    }

    // Step 16: Exit with error code if errors were found
    if !result.errors.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}
