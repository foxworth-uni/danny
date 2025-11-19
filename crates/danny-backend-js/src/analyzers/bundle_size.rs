//! Bundle size impact analysis for unreachable modules.

use danny_core::types::{BundleSizeImpact, ModuleSizeInfo};
use std::path::PathBuf;

/// Analyzes bundle size impact of dead code
pub struct BundleSizeAnalyzer;

impl BundleSizeAnalyzer {
    /// Calculates total potential savings from unreachable modules
    ///
    /// # Arguments
    /// * `modules` - List of (path, size, has_side_effects) tuples
    ///
    /// # Returns
    /// Bundle size impact breakdown
    pub fn calculate_impact(modules: Vec<(PathBuf, usize, bool)>) -> BundleSizeImpact {
        let mut total_savings = 0;
        let mut safe_savings = 0;
        let mut by_module = Vec::new();

        for (path, size, has_side_effects) in modules {
            total_savings += size;

            if !has_side_effects {
                safe_savings += size;
            }

            by_module.push(ModuleSizeInfo {
                path,
                size_bytes: size,
                has_side_effects,
            });
        }

        // Sort by size descending for reporting
        by_module.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        BundleSizeImpact {
            total_savings_bytes: total_savings,
            safe_savings_bytes: safe_savings,
            by_module,
        }
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
    fn test_calculate_impact() {
        let modules = vec![
            (PathBuf::from("a.ts"), 1000, false),
            (PathBuf::from("b.ts"), 2000, true),
            (PathBuf::from("c.ts"), 500, false),
        ];

        let impact = BundleSizeAnalyzer::calculate_impact(modules);

        assert_eq!(impact.total_savings_bytes, 3500);
        assert_eq!(impact.safe_savings_bytes, 1500);
        assert_eq!(impact.by_module.len(), 3);
        // Verify sorted by size descending
        assert_eq!(impact.by_module[0].size_bytes, 2000);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(BundleSizeAnalyzer::format_bytes(500), "500 bytes");
        assert_eq!(BundleSizeAnalyzer::format_bytes(1536), "1.5 KB");
        assert_eq!(BundleSizeAnalyzer::format_bytes(2_097_152), "2.00 MB");
    }

    #[test]
    fn test_empty_modules() {
        let impact = BundleSizeAnalyzer::calculate_impact(vec![]);
        assert_eq!(impact.total_savings_bytes, 0);
        assert_eq!(impact.safe_savings_bytes, 0);
        assert!(impact.by_module.is_empty());
    }

    #[cfg(feature = "property-tests")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        fn module_strategy() -> impl Strategy<Value = (PathBuf, usize, bool)> {
            (
                (0usize..1000).prop_map(|i| PathBuf::from(format!("module_{}.ts", i))),
                0usize..1_000_000,
                any::<bool>(),
            )
        }

        proptest! {
            #[test]
            fn test_bundle_size_never_negative(
                sizes in prop::collection::vec(0usize..1_000_000, 0..100),
                side_effects in prop::collection::vec(any::<bool>(), 0..100)
            ) {
                let modules: Vec<_> = sizes
                    .iter()
                    .zip(side_effects.iter())
                    .enumerate()
                    .map(|(i, (&size, &has_side_effects))| {
                        (PathBuf::from(format!("mod{}.ts", i)), size, has_side_effects)
                    })
                    .collect();

                let impact = BundleSizeAnalyzer::calculate_impact(modules);

                prop_assert!(impact.total_savings_bytes >= 0);
                prop_assert!(impact.safe_savings_bytes >= 0);
                prop_assert!(impact.safe_savings_bytes <= impact.total_savings_bytes);
            }

            #[test]
            fn test_bundle_size_safe_never_exceeds_total(
                modules in prop::collection::vec(module_strategy(), 0..50)
            ) {
                let impact = BundleSizeAnalyzer::calculate_impact(modules);

                prop_assert!(impact.safe_savings_bytes <= impact.total_savings_bytes);
            }

            #[test]
            fn test_bundle_size_sorted_descending(
                modules in prop::collection::vec(module_strategy(), 1..20)
            ) {
                let impact = BundleSizeAnalyzer::calculate_impact(modules);

                // Verify sorted by size descending
                for i in 0..(impact.by_module.len().saturating_sub(1)) {
                    prop_assert!(
                        impact.by_module[i].size_bytes >= impact.by_module[i + 1].size_bytes,
                        "Modules not sorted descending: {} >= {}",
                        impact.by_module[i].size_bytes,
                        impact.by_module[i + 1].size_bytes
                    );
                }
            }

            #[test]
            fn test_bundle_size_sum_matches(
                modules in prop::collection::vec(module_strategy(), 0..30)
            ) {
                let impact = BundleSizeAnalyzer::calculate_impact(modules.clone());

                let expected_total: usize = modules.iter().map(|(_, size, _)| size).sum();
                let expected_safe: usize = modules
                    .iter()
                    .filter(|(_, _, has_side_effects)| !has_side_effects)
                    .map(|(_, size, _)| size)
                    .sum();

                prop_assert_eq!(impact.total_savings_bytes, expected_total);
                prop_assert_eq!(impact.safe_savings_bytes, expected_safe);
                prop_assert_eq!(impact.by_module.len(), modules.len());
            }
        }
    }
}
