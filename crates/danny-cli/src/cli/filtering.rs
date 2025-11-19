//! Filtering logic for findings.

use danny_core::{Finding, IgnoredFindingsBreakdown};

/// Result of filtering findings.
#[derive(Debug)]
pub struct FilterResult {
    /// Findings that passed the filter.
    pub kept: Vec<Finding>,

    /// Findings that were filtered out with metadata.
    pub ignored: Vec<danny_core::IgnoredFinding>,
}

/// Filters findings and tracks what was ignored.
pub fn filter_findings_with_tracking(
    findings: Vec<Finding>,
    ignore_set: &globset::GlobSet,
    pattern_infos: &[crate::ignore::PatternInfo],
) -> FilterResult {
    let mut kept = Vec::new();
    let mut ignored = Vec::new();

    for finding in findings {
        let path_to_check = match &finding {
            Finding::Module { path, .. } => Some(path),
            Finding::Dependency { from, .. } => Some(from),
            Finding::Pattern { location, .. } => Some(location),
            Finding::Framework { .. } => None,
            Finding::UnusedExport {
                module,
                explanation: _,
                ..
            } => Some(module),
            Finding::UnreachableModule { path, .. } => Some(path),
            Finding::UnreachableFile {
                path,
                explanation: _,
                ..
            } => Some(path),
            Finding::UnusedSymbol {
                module,
                explanation: _,
                ..
            } => Some(module),
            Finding::FrameworkExport {
                module,
                explanation: _,
                ..
            } => Some(module),
            Finding::DynamicImport(info) => Some(&info.from),
            Finding::CircularDependency(circ) => circ.cycle.first(),
            Finding::UnusedPrivateClassMember { module, .. } => Some(module),
            Finding::UnusedPublicClassMember { module, .. } => Some(module),
            Finding::UnusedEnumMember { module, .. } => Some(module),
            Finding::UnusedNpmDependency { .. } => None,
            Finding::SideEffectOnlyImport { module, .. } => Some(module),
            Finding::NamespaceImport { module, .. } => Some(module),
            Finding::TypeOnlyImport { module, .. } => Some(module),
            Finding::DeadCodeModule { path, .. } => Some(path),
            Finding::DependencyChain { chain, .. } => chain.first(),
            Finding::CodeSmell { location, .. } => Some(location),
        };

        match path_to_check {
            Some(path) => {
                if let Some(matched_pattern) =
                    crate::ignore::match_with_pattern(path, ignore_set, pattern_infos)
                {
                    // This finding was filtered - track it
                    ignored.push(danny_core::IgnoredFinding {
                        finding: finding.clone(),
                        matched_pattern,
                        matched_path: path.clone(),
                    });
                } else {
                    kept.push(finding);
                }
            }
            None => {
                // No path to check - keep the finding
                kept.push(finding);
            }
        }
    }

    FilterResult { kept, ignored }
}

/// Calculates ignore statistics from ignored findings.
pub fn calculate_ignore_statistics(
    ignored: &[danny_core::IgnoredFinding],
) -> IgnoredFindingsBreakdown {
    let mut breakdown = IgnoredFindingsBreakdown::default();

    for ignored_finding in ignored {
        match ignored_finding.finding {
            Finding::UnusedExport { .. } => breakdown.unused_exports += 1,
            Finding::UnreachableModule { .. } => breakdown.unreachable_modules += 1,
            Finding::UnreachableFile { .. } => breakdown.unreachable_files += 1,
            Finding::UnusedSymbol { .. } => breakdown.unused_symbols += 1,
            Finding::FrameworkExport { .. } => breakdown.framework_exports += 1,
            Finding::Module { .. } => breakdown.modules += 1,
            Finding::Dependency { .. } => breakdown.dependencies += 1,
            Finding::Pattern { .. } => breakdown.patterns += 1,
            _ => {}
        }
    }

    breakdown
}

/// Recalculates statistics after filtering findings.
pub fn recalculate_statistics(result: &mut danny_core::AnalysisResult) {
    let mut unused_exports = 0;
    let mut unreachable_modules = 0;
    let mut framework_exports = 0;

    for finding in &result.findings {
        match finding {
            Finding::UnusedExport { .. } => unused_exports += 1,
            Finding::UnreachableModule { .. } => unreachable_modules += 1,
            Finding::FrameworkExport { .. } => framework_exports += 1,
            Finding::DynamicImport(_) | Finding::CircularDependency(_) => {
                // These are counted in statistics, not here
            }
            _ => {}
        }
    }

    result.statistics.unused_exports_count = unused_exports;
    result.statistics.unreachable_modules_count = unreachable_modules;
    result.statistics.framework_exports_count = framework_exports;
}
