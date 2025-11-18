//! Enrichment pipeline for Phase 1 features.
//!
//! This module coordinates the enrichment of findings with additional metadata
//! from side-effect analysis, bundle size calculation, etc.

use crate::types::{BundleSizeImpact, ModuleSizeInfo};
use crate::error::Result;
use std::path::PathBuf;
use std::collections::HashMap;

/// Enriches findings with Phase 1 metadata
pub struct FindingEnricher {
    modules_metadata: HashMap<PathBuf, ModuleMetadata>,
}

#[derive(Debug, Clone)]
struct ModuleMetadata {
    size: usize,
    has_side_effects: bool,
}

impl FindingEnricher {
    /// Creates new enricher from module data
    pub fn new(modules: Vec<(PathBuf, usize, bool)>) -> Self {
        let modules_metadata = modules
            .into_iter()
            .map(|(path, size, side_effects)| {
                (
                    path,
                    ModuleMetadata {
                        size,
                        has_side_effects: side_effects,
                    },
                )
            })
            .collect();

        Self { modules_metadata }
    }

    /// Calculates bundle size impact from unreachable modules
    pub fn calculate_bundle_impact(
        &self,
        unreachable_paths: &[PathBuf],
    ) -> Result<BundleSizeImpact> {
        let mut total_savings = 0;
        let mut safe_savings = 0;
        let mut by_module = Vec::new();

        for path in unreachable_paths {
            if let Some(metadata) = self.modules_metadata.get(path) {
                total_savings += metadata.size;

                if !metadata.has_side_effects {
                    safe_savings += metadata.size;
                }

                by_module.push(ModuleSizeInfo {
                    path: path.clone(),
                    size_bytes: metadata.size,
                    has_side_effects: metadata.has_side_effects,
                });
            }
        }

        // Sort by size descending
        by_module.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        Ok(BundleSizeImpact {
            total_savings_bytes: total_savings,
            safe_savings_bytes: safe_savings,
            by_module,
        })
    }

    /// Formats bytes into human-readable string
    pub fn format_bytes(bytes: usize) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_impact_calculation() {
        let enricher = FindingEnricher::new(vec![
            (PathBuf::from("a.ts"), 1000, false),
            (PathBuf::from("b.ts"), 2000, true),
            (PathBuf::from("c.ts"), 500, false),
        ]);

        let unreachable = vec![
            PathBuf::from("a.ts"),
            PathBuf::from("b.ts"),
            PathBuf::from("c.ts"),
        ];

        let impact = enricher.calculate_bundle_impact(&unreachable).unwrap();

        assert_eq!(impact.total_savings_bytes, 3500);
        assert_eq!(impact.safe_savings_bytes, 1500); // a + c (no side effects)
        assert_eq!(impact.by_module.len(), 3);
        // Verify sorted by size descending
        assert_eq!(impact.by_module[0].size_bytes, 2000);
        assert_eq!(impact.by_module[1].size_bytes, 1000);
        assert_eq!(impact.by_module[2].size_bytes, 500);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(FindingEnricher::format_bytes(500), "500 bytes");
        assert_eq!(FindingEnricher::format_bytes(1536), "1.5 KB");
        assert_eq!(FindingEnricher::format_bytes(2_097_152), "2.00 MB");
    }
}

