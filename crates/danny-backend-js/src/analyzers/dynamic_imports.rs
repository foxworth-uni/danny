//! Dynamic import analysis for code-splitting detection.

use danny_core::types::DynamicImportInfo;
use std::collections::HashMap;
use std::path::PathBuf;

/// Analyzes dynamic imports for code-splitting opportunities
pub struct DynamicImportAnalyzer;

impl DynamicImportAnalyzer {
    /// Extracts all dynamic imports from import data
    ///
    /// # Arguments
    /// * `imports` - List of (from_path, to_path, source, is_external) tuples
    ///
    /// # Returns
    /// Vector of dynamic import information
    pub fn extract_dynamic_imports(
        imports: Vec<(PathBuf, PathBuf, String, bool)>,
    ) -> Vec<DynamicImportInfo> {
        imports
            .into_iter()
            .map(|(from, to, source, is_external)| DynamicImportInfo {
                from,
                to,
                source,
                creates_chunk: !is_external,
            })
            .collect()
    }

    /// Groups dynamic imports by parent module
    pub fn group_by_parent(
        imports: Vec<DynamicImportInfo>,
    ) -> HashMap<PathBuf, Vec<DynamicImportInfo>> {
        let mut grouped: HashMap<PathBuf, Vec<DynamicImportInfo>> = HashMap::new();

        for import in imports {
            grouped.entry(import.from.clone()).or_default().push(import);
        }

        grouped
    }

    /// Calculates potential bundle savings from lazy loading
    pub fn calculate_lazy_loading_potential(
        imports: &[DynamicImportInfo],
        module_sizes: &HashMap<PathBuf, usize>,
    ) -> usize {
        imports
            .iter()
            .filter(|imp| imp.creates_chunk)
            .filter_map(|imp| module_sizes.get(&imp.to))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dynamic_imports() {
        let imports = vec![(
            PathBuf::from("app.ts"),
            PathBuf::from("dashboard.ts"),
            "./dashboard".into(),
            false,
        )];

        let dynamics = DynamicImportAnalyzer::extract_dynamic_imports(imports);

        assert_eq!(dynamics.len(), 1);
        assert_eq!(dynamics[0].source, "./dashboard");
        assert!(dynamics[0].creates_chunk);
    }

    #[test]
    fn test_group_by_parent() {
        let imports = vec![
            DynamicImportInfo {
                from: PathBuf::from("a.ts"),
                to: PathBuf::from("b.ts"),
                source: "./b".into(),
                creates_chunk: true,
            },
            DynamicImportInfo {
                from: PathBuf::from("a.ts"),
                to: PathBuf::from("c.ts"),
                source: "./c".into(),
                creates_chunk: true,
            },
        ];

        let grouped = DynamicImportAnalyzer::group_by_parent(imports);

        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped.get(&PathBuf::from("a.ts")).unwrap().len(), 2);
    }

    #[test]
    fn test_lazy_loading_potential() {
        let imports = vec![
            DynamicImportInfo {
                from: PathBuf::from("app.ts"),
                to: PathBuf::from("page1.ts"),
                source: "./page1".into(),
                creates_chunk: true,
            },
            DynamicImportInfo {
                from: PathBuf::from("app.ts"),
                to: PathBuf::from("page2.ts"),
                source: "./page2".into(),
                creates_chunk: true,
            },
        ];

        let mut sizes = HashMap::new();
        sizes.insert(PathBuf::from("page1.ts"), 10000);
        sizes.insert(PathBuf::from("page2.ts"), 15000);

        let potential = DynamicImportAnalyzer::calculate_lazy_loading_potential(&imports, &sizes);

        assert_eq!(potential, 25000);
    }
}
