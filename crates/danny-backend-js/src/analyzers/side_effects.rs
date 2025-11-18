//! Side-effect analysis for unreachable modules.
//!
//! Determines whether an unreachable module is safe to delete based on:
//! - Side-effect detection from Fob
//! - Entry point status
//! - Dynamic import targets
//! - Configuration file patterns

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use danny_core::types::SafetyAssessment;

/// Analyzes modules for side effects and deletion safety
pub struct SideEffectAnalyzer;

impl SideEffectAnalyzer {
    /// Determines if a module is safe to delete
    ///
    /// # Arguments
    /// * `has_side_effects` - From Fob's Module.has_side_effects
    /// * `is_entry` - Whether module is an entry point
    /// * `path` - Module path
    /// * `dynamic_imports` - Set of dynamically imported modules
    ///
    /// # Returns
    /// Safety assessment with reasoning
    pub fn assess_safety(
        has_side_effects: bool,
        is_entry: bool,
        path: &Path,
        dynamic_imports: &HashSet<PathBuf>,
    ) -> SafetyAssessment {
        if is_entry {
            return SafetyAssessment::Unsafe("Entry point".into());
        }

        if Self::is_config_file(path) {
            return SafetyAssessment::Unsafe("Configuration file".into());
        }

        if dynamic_imports.contains(path) {
            return SafetyAssessment::ReviewCarefully(
                "Dynamically imported".into()
            );
        }

        if has_side_effects {
            return SafetyAssessment::ReviewCarefully(
                "Module has side effects".into()
            );
        }

        SafetyAssessment::SafeToDelete
    }

    /// Checks if a file is a configuration file
    fn is_config_file(path: &Path) -> bool {
        const CONFIG_FILES: &[&str] = &[
            "package.json",
            "tsconfig.json",
            "jsconfig.json",
            ".eslintrc",
            ".prettierrc",
            "next.config",
            "vite.config",
            "rollup.config",
        ];

        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| {
                CONFIG_FILES.iter().any(|cfg| name.contains(cfg))
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_to_delete_no_side_effects() {
        let path = PathBuf::from("src/utils.ts");
        let assessment = SideEffectAnalyzer::assess_safety(
            false,  // no side effects
            false,  // not entry
            &path,
            &HashSet::new(),
        );

        assert_eq!(assessment, SafetyAssessment::SafeToDelete);
    }

    #[test]
    fn test_unsafe_entry_point() {
        let path = PathBuf::from("src/index.ts");
        let assessment = SideEffectAnalyzer::assess_safety(
            false,
            true,  // is entry
            &path,
            &HashSet::new(),
        );

        assert!(matches!(assessment, SafetyAssessment::Unsafe(_)));
    }

    #[test]
    fn test_review_carefully_side_effects() {
        let path = PathBuf::from("src/setup.ts");
        let assessment = SideEffectAnalyzer::assess_safety(
            true,  // has side effects
            false,
            &path,
            &HashSet::new(),
        );

        assert!(matches!(assessment, SafetyAssessment::ReviewCarefully(_)));
    }

    #[test]
    fn test_review_carefully_dynamic_import() {
        let path = PathBuf::from("src/dynamic.ts");
        let mut dynamic_imports = HashSet::new();
        dynamic_imports.insert(path.clone());

        let assessment = SideEffectAnalyzer::assess_safety(
            false,
            false,
            &path,
            &dynamic_imports,
        );

        assert!(matches!(assessment, SafetyAssessment::ReviewCarefully(_)));
    }

    #[test]
    fn test_unsafe_config_file() {
        let path = PathBuf::from("package.json");
        let assessment = SideEffectAnalyzer::assess_safety(
            false,
            false,
            &path,
            &HashSet::new(),
        );

        assert!(matches!(assessment, SafetyAssessment::Unsafe(_)));
    }

    #[test]
    fn test_is_config_file_detection() {
        assert!(SideEffectAnalyzer::is_config_file(Path::new("package.json")));
        assert!(SideEffectAnalyzer::is_config_file(Path::new("tsconfig.json")));
        assert!(SideEffectAnalyzer::is_config_file(Path::new("next.config.js")));
        assert!(!SideEffectAnalyzer::is_config_file(Path::new("src/config.ts")));
    }
}

