//! NPM dependency analysis - converts Fob's npm dependency data to Danny findings.

use danny_core::{Finding, NpmDependencyType};
use fob::graph::{package_json::DependencyCoverage, package_json::UnusedDependency};

/// Analyzer for npm dependency findings.
pub struct NpmDependencyAnalyzer;

impl NpmDependencyAnalyzer {
    /// Convert Fob's unused npm dependencies to Danny findings.
    pub fn convert_unused_dependencies(unused: &[UnusedDependency]) -> Vec<Finding> {
        unused
            .iter()
            .map(|dep| Finding::UnusedNpmDependency {
                package: dep.package.clone(),
                version: dep.version.clone(),
                dep_type: Self::convert_dependency_type(&dep.dep_type),
            })
            .collect()
    }

    /// Convert Fob's dependency coverage to Danny statistics.
    pub fn convert_coverage_stats(
        coverage: &DependencyCoverage,
    ) -> danny_core::DependencyCoverageStats {
        use danny_core::TypeCoverage;

        let by_type: Vec<(NpmDependencyType, TypeCoverage)> = coverage
            .by_type
            .iter()
            .map(|(dep_type, type_cov)| {
                (
                    Self::convert_dependency_type(dep_type),
                    TypeCoverage {
                        declared: type_cov.declared,
                        used: type_cov.used,
                        unused: type_cov.unused,
                    },
                )
            })
            .collect();

        danny_core::DependencyCoverageStats {
            total_declared: coverage.total_declared,
            total_used: coverage.total_used,
            total_unused: coverage.total_unused,
            coverage_percentage: coverage.coverage_percentage(),
            by_type,
        }
    }

    /// Convert Fob's DependencyType to Danny's NpmDependencyType.
    fn convert_dependency_type(
        dep_type: &fob::graph::package_json::DependencyType,
    ) -> NpmDependencyType {
        match dep_type {
            fob::graph::package_json::DependencyType::Production => NpmDependencyType::Production,
            fob::graph::package_json::DependencyType::Development => NpmDependencyType::Development,
            fob::graph::package_json::DependencyType::Peer => NpmDependencyType::Peer,
            fob::graph::package_json::DependencyType::Optional => NpmDependencyType::Optional,
        }
    }
}
