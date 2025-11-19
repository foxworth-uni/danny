use danny_config::AnalysisTarget;
use danny_core::{AnalysisCapabilities, AnalysisMode, Category};

/// Validates requested categories against available capabilities
pub struct CategoryValidator {
    capabilities: AnalysisCapabilities,
}

impl CategoryValidator {
    /// Create validator for an analysis target
    pub fn new(target: &AnalysisTarget) -> Self {
        Self {
            capabilities: target.capabilities(),
        }
    }

    /// Validate requested categories
    pub fn validate(&self, requested: &[Category]) -> CategoryValidation {
        // No categories requested - use defaults
        if requested.is_empty() {
            return CategoryValidation::UseDefaults {
                categories: self.default_categories(),
            };
        }

        let available: Vec<Category> = self.capabilities.filter_available(requested).collect();
        let unavailable: Vec<Category> = self.capabilities.filter_unavailable(requested).collect();

        if unavailable.is_empty() {
            // All requested categories available
            CategoryValidation::AllAvailable {
                categories: available,
            }
        } else if available.is_empty() {
            // None of the requested categories available
            CategoryValidation::NoneAvailable {
                requested: requested.to_vec(),
                unavailable,
            }
        } else {
            // Some available, some not
            CategoryValidation::PartiallyAvailable {
                available,
                unavailable,
            }
        }
    }

    /// Get default categories for the mode
    fn default_categories(&self) -> Vec<Category> {
        match self.capabilities.mode() {
            AnalysisMode::Package => {
                // Return ALL available categories for comprehensive analysis
                self.capabilities
                    .available_categories()
                    .iter()
                    .copied()
                    .collect()
            }
            AnalysisMode::Files => vec![
                Category::Symbols,
                Category::Quality,
                Category::Imports,
                Category::Types,
            ],
        }
    }

    /// Get capabilities
    pub fn capabilities(&self) -> &AnalysisCapabilities {
        &self.capabilities
    }
}

/// Result of category validation
#[derive(Debug)]
pub enum CategoryValidation {
    /// No categories specified, using defaults
    UseDefaults { categories: Vec<Category> },

    /// All requested categories are available
    AllAvailable { categories: Vec<Category> },

    /// Some categories available, some not
    PartiallyAvailable {
        available: Vec<Category>,
        unavailable: Vec<Category>,
    },

    /// None of the requested categories are available
    NoneAvailable {
        requested: Vec<Category>,
        unavailable: Vec<Category>,
    },
}

impl CategoryValidation {
    /// Check if this requires user confirmation
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, CategoryValidation::PartiallyAvailable { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use danny_config::{FilesTarget, PackageTarget};
    use std::path::PathBuf;

    #[test]
    fn test_validate_all_available_package_mode() {
        let target = AnalysisTarget::Package(PackageTarget {
            root: PathBuf::from("/project"),
            entry_points: vec![],
            framework: None,
            has_node_modules: true,
        });

        let validator = CategoryValidator::new(&target);
        let result = validator.validate(&[Category::Files, Category::Exports]);

        match result {
            CategoryValidation::AllAvailable { categories } => {
                assert_eq!(categories.len(), 2);
            }
            _ => panic!("Expected AllAvailable"),
        }
    }

    #[test]
    fn test_validate_none_available_files_mode() {
        let target = AnalysisTarget::Files(FilesTarget {
            files: vec![PathBuf::from("foo.ts")],
            working_dir: PathBuf::from("/project"),
            nearby_package: None,
        });

        let validator = CategoryValidator::new(&target);
        let result = validator.validate(&[Category::Files, Category::Circular]);

        match result {
            CategoryValidation::NoneAvailable { unavailable, .. } => {
                assert_eq!(unavailable.len(), 2);
            }
            _ => panic!("Expected NoneAvailable"),
        }
    }

    #[test]
    fn test_validate_partial_files_mode() {
        let target = AnalysisTarget::Files(FilesTarget {
            files: vec![PathBuf::from("foo.ts")],
            working_dir: PathBuf::from("/project"),
            nearby_package: Some(PathBuf::from("/project/package.json")),
        });

        let validator = CategoryValidator::new(&target);
        let result = validator.validate(&[
            Category::Symbols, // Available
            Category::Files,   // Not available
            Category::Quality, // Available
        ]);

        match result {
            CategoryValidation::PartiallyAvailable {
                available,
                unavailable,
            } => {
                assert_eq!(available.len(), 2);
                assert_eq!(unavailable.len(), 1);
            }
            _ => panic!("Expected PartiallyAvailable"),
        }
    }

    #[test]
    fn test_defaults_package_mode() {
        let target = AnalysisTarget::Package(PackageTarget {
            root: PathBuf::from("/project"),
            entry_points: vec![],
            framework: None,
            has_node_modules: true,
        });

        let validator = CategoryValidator::new(&target);
        let result = validator.validate(&[]);

        match result {
            CategoryValidation::UseDefaults { categories } => {
                assert!(categories.contains(&Category::Files));
                assert!(categories.contains(&Category::Exports));
                assert!(categories.contains(&Category::Dependencies));
            }
            _ => panic!("Expected UseDefaults"),
        }
    }

    #[test]
    fn test_defaults_files_mode() {
        let target = AnalysisTarget::Files(FilesTarget {
            files: vec![PathBuf::from("foo.ts")],
            working_dir: PathBuf::from("/project"),
            nearby_package: None,
        });

        let validator = CategoryValidator::new(&target);
        let result = validator.validate(&[]);

        match result {
            CategoryValidation::UseDefaults { categories } => {
                assert!(categories.contains(&Category::Symbols));
                assert!(categories.contains(&Category::Quality));
                assert!(!categories.contains(&Category::Files));
            }
            _ => panic!("Expected UseDefaults"),
        }
    }
}
