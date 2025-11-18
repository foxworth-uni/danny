//! Type-only import/export analysis for TypeScript.

use std::path::PathBuf;

/// Analyzes and categorizes exports by runtime impact
pub struct TypeOnlyAnalyzer;

/// Categorized unused exports
#[derive(Debug, Clone)]
pub struct CategorizedExports {
    /// Type-only exports (no runtime impact)
    pub type_only: Vec<UnusedExport>,
    /// Runtime exports (actual code)
    pub runtime: Vec<UnusedExport>,
}

#[derive(Debug, Clone)]
pub struct UnusedExport {
    pub module: PathBuf,
    pub name: String,
    pub is_type_only: bool,
}

impl TypeOnlyAnalyzer {
    /// Categorizes unused exports by runtime impact
    pub fn categorize_exports(
        unused_exports: Vec<UnusedExport>,
    ) -> CategorizedExports {
        let mut type_only = Vec::new();
        let mut runtime = Vec::new();

        for export in unused_exports {
            if export.is_type_only {
                type_only.push(export);
            } else {
                runtime.push(export);
            }
        }

        CategorizedExports { type_only, runtime }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_type_only() {
        let exports = vec![
            UnusedExport {
                module: PathBuf::from("types.ts"),
                name: "UserType".into(),
                is_type_only: true,
            },
            UnusedExport {
                module: PathBuf::from("utils.ts"),
                name: "calculate".into(),
                is_type_only: false,
            },
        ];

        let categorized = TypeOnlyAnalyzer::categorize_exports(exports);

        assert_eq!(categorized.type_only.len(), 1);
        assert_eq!(categorized.runtime.len(), 1);
        assert_eq!(categorized.type_only[0].name, "UserType");
        assert_eq!(categorized.runtime[0].name, "calculate");
    }

    #[test]
    fn test_all_runtime() {
        let exports = vec![
            UnusedExport {
                module: PathBuf::from("a.ts"),
                name: "foo".into(),
                is_type_only: false,
            },
        ];

        let categorized = TypeOnlyAnalyzer::categorize_exports(exports);

        assert_eq!(categorized.type_only.len(), 0);
        assert_eq!(categorized.runtime.len(), 1);
    }
}

