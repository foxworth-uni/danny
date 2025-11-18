//! Human-readable formatter for analysis results.

mod files;
mod exports;
mod types;
mod symbols;
mod dependencies;
mod imports;
mod circular;
mod quality;
mod framework;

#[cfg(test)]
mod tests;

use danny_core::{AnalysisResult, Finding, Category};
use danny_core::types::CodeSmellType;
use std::collections::HashMap;

pub struct HumanFormatter;

pub fn print_results(result: &AnalysisResult) {
    println!("\nDanny Analysis Results");
    println!("======================\n");

    println!("Statistics:");
    println!("  Total modules: {}", result.statistics.total_modules);
    println!(
        "  Total dependencies: {}",
        result.statistics.total_dependencies
    );
    println!(
        "  External dependencies: {}",
        result.statistics.external_dependencies
    );
    println!(
        "  Frameworks detected: {}",
        result.statistics.frameworks_detected.join(", ")
    );
    println!("  Duration: {}ms\n", result.statistics.duration_ms);

    // Group findings by category
    let mut findings_by_category: HashMap<Category, Vec<&Finding>> = HashMap::new();
    for finding in &result.findings {
        let category = finding.category();
        findings_by_category.entry(category).or_default().push(finding);
    }

    // Print summary statistics by category
    println!("Findings by Category:");
    for category in Category::all() {
        if let Some(findings) = findings_by_category.get(category) {
            if !findings.is_empty() {
                println!("  {}: {}", category.display_name(), findings.len());
            }
        }
    }

    // Print detailed findings grouped by category
    println!();
    for category in Category::all() {
        if let Some(findings) = findings_by_category.get(category) {
            if !findings.is_empty() {
                print_category_findings(*category, findings, result);
            }
        }
    }

    // Print code quality statistics if available
    if let Some(stats) = &result.statistics.code_quality_stats {
        println!("\nCode Quality Statistics:");
        println!("  Total code smells: {}", stats.total_smells);
        if !stats.by_type.is_empty() {
            println!("  By type:");
            for (smell_type, count) in &stats.by_type {
                let type_name = match smell_type {
                    CodeSmellType::LongFunction => "Long Function",
                    CodeSmellType::TooManyParameters => "Too Many Parameters",
                    CodeSmellType::LargeClass => "Large Class",
                    CodeSmellType::MagicNumber => "Magic Number",
                    CodeSmellType::MessageChain => "Message Chain",
                    CodeSmellType::ComplexConditional => "Complex Conditional",
                    CodeSmellType::DeepNesting => "Deep Nesting",
                    CodeSmellType::MultipleReturns => "Multiple Returns",
                    CodeSmellType::EmptyCatchBlock => "Empty Catch Block",
                    CodeSmellType::DuplicatedCode => "Duplicated Code",
                    CodeSmellType::LongParameterList => "Long Parameter List",
                    CodeSmellType::TooManyMethods => "Too Many Methods",
                    CodeSmellType::TooManyFields => "Too Many Fields",
                    CodeSmellType::LowCohesion => "Low Cohesion",
                };
                println!("    {}: {}", type_name, count);
            }
        }
    }

    // Print class member statistics if available
    if let Some(stats) = &result.statistics.class_member_stats {
        println!(
            "  Class members: {} total ({} unused private, {} unused public)",
            stats.total_members,
            stats.unused_private,
            stats.unused_public
        );
    }

    // Print enum statistics if available
    if let Some(stats) = &result.statistics.enum_stats {
        println!(
            "  Enums: {} total ({} members, {} unused)",
            stats.total_enums,
            stats.total_members,
            stats.unused_members
        );
    }

    // Print dependency coverage statistics if available
    if let Some(stats) = &result.statistics.dependency_coverage_stats {
        println!(
            "  Dependency coverage: {:.1}% ({} used / {} declared)",
            stats.coverage_percentage,
            stats.total_used,
            stats.total_declared
        );
    }

    // Print symbol statistics if available
    if let Some(symbol_stats) = &result.statistics.symbol_statistics {
        println!(
            "  Total symbols: {} ({} unused)",
            symbol_stats.total_symbols,
            symbol_stats.unused_symbols
        );
    }

    println!();
}

/// Prints findings for a specific category.
fn print_category_findings(category: Category, findings: &[&Finding], result: &AnalysisResult) {
    match category {
        Category::Files => files::print_files(findings, result),
        Category::Exports => exports::print_exports(findings),
        Category::Types => types::print_types(findings),
        Category::Symbols => symbols::print_symbols(findings),
        Category::Dependencies => dependencies::print_dependencies(findings),
        Category::Imports => imports::print_imports(findings),
        Category::Circular => circular::print_circular(findings),
        Category::Quality => quality::print_quality(findings),
        Category::Framework => framework::print_framework(findings),
    }
}

pub(crate) fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

