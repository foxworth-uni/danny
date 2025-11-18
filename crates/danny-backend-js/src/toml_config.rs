//! TOML configuration types for Danny.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Danny configuration loaded from `.danny.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DannyConfig {
    /// Framework-specific configuration.
    #[serde(default)]
    pub frameworks: HashMap<String, FrameworkConfig>,

    /// Entry point configuration.
    #[serde(default)]
    pub entry_points: EntryPointConfig,

    /// Analysis options.
    #[serde(default)]
    pub analysis: AnalysisConfig,

    /// Code quality configuration.
    #[serde(default)]
    pub quality: CodeQualityConfig,
}

/// Configuration for a specific framework.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkConfig {
    /// Whether this framework detection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Glob patterns for entry points (e.g., pages, routes).
    #[serde(default)]
    pub entry_patterns: Vec<String>,

    /// Glob patterns for components.
    #[serde(default)]
    pub component_patterns: Vec<String>,

    /// Patterns to exclude from detection.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Minimum confidence threshold (0.0 to 1.0).
    #[serde(default = "default_confidence")]
    pub confidence_threshold: f32,
}

/// Entry point detection configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EntryPointConfig {
    /// Auto-detect entry points from package.json.
    #[serde(default = "default_true")]
    pub auto_detect: bool,

    /// Manually specified entry points.
    #[serde(default)]
    pub manual: Vec<String>,

    /// Glob patterns for entry points.
    #[serde(default)]
    pub patterns: Vec<String>,

    /// Patterns to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Analysis behavior configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisConfig {
    /// Follow external (npm) dependencies.
    #[serde(default)]
    pub follow_external: bool,

    /// Maximum traversal depth (None = unlimited).
    #[serde(default)]
    pub max_depth: Option<usize>,

    /// Number of parallel workers (None = auto).
    #[serde(default)]
    pub workers: Option<usize>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            follow_external: false,
            max_depth: None,
            workers: None,
        }
    }
}

/// Code quality analysis configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeQualityConfig {
    /// Maximum function lines before flagging as long function.
    #[serde(default = "default_max_function_lines")]
    pub max_function_lines: usize,

    /// Maximum parameters before flagging as too many parameters.
    #[serde(default = "default_max_parameters")]
    pub max_parameters: usize,

    /// Maximum class lines before flagging as large class.
    #[serde(default = "default_max_class_lines")]
    pub max_class_lines: usize,

    /// Maximum cyclomatic complexity before flagging.
    #[serde(default = "default_max_complexity")]
    pub max_complexity: usize,

    /// Maximum nesting depth before flagging.
    #[serde(default = "default_max_nesting")]
    pub max_nesting: usize,

    /// Maximum message chain length before flagging.
    #[serde(default = "default_max_message_chain")]
    pub max_message_chain: usize,

    /// Maximum methods per class before flagging.
    #[serde(default = "default_max_methods")]
    pub max_methods: usize,

    /// Maximum fields per class before flagging.
    #[serde(default = "default_max_fields")]
    pub max_fields: usize,

    /// Maximum return statements before flagging.
    #[serde(default = "default_max_return_count")]
    pub max_return_count: usize,
}

impl Default for CodeQualityConfig {
    fn default() -> Self {
        Self {
            max_function_lines: 50,
            max_parameters: 4,
            max_class_lines: 300,
            max_complexity: 10,
            max_nesting: 4,
            max_message_chain: 4,
            max_methods: 20,
            max_fields: 10,
            max_return_count: 3,
        }
    }
}

impl CodeQualityConfig {
    /// Validates the configuration values to ensure they are within acceptable ranges.
    ///
    /// This method checks that all threshold values are:
    /// 1. Greater than zero (preventing divide-by-zero and nonsensical thresholds)
    /// 2. Within reasonable upper bounds (preventing misconfiguration)
    ///
    /// # Returns
    /// `Ok(())` if validation passes, or an error message describing the issue.
    ///
    /// # Errors
    /// Returns an error string if any configuration value is invalid.
    ///
    /// # Design Rationale
    /// Configuration validation at load time (fail-fast) prevents runtime errors
    /// and provides clear feedback to users about configuration mistakes. The
    /// upper bounds are generous to accommodate legitimate edge cases while
    /// catching obvious typos (e.g., 50000 instead of 500).
    ///
    /// # Examples
    /// ```
    /// use danny_backend_js::toml_config::CodeQualityConfig;
    ///
    /// let valid_config = CodeQualityConfig::default();
    /// assert!(valid_config.validate().is_ok());
    ///
    /// let invalid_config = CodeQualityConfig {
    ///     max_function_lines: 0,
    ///     ..Default::default()
    /// };
    /// assert!(invalid_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // Validate max_function_lines
        if self.max_function_lines == 0 {
            return Err("max_function_lines must be > 0".to_string());
        }
        if self.max_function_lines > 10000 {
            return Err("max_function_lines unreasonably large (>10000)".to_string());
        }

        // Validate max_parameters
        if self.max_parameters == 0 {
            return Err("max_parameters must be > 0".to_string());
        }
        if self.max_parameters > 20 {
            return Err("max_parameters unreasonably large (>20)".to_string());
        }

        // Validate max_class_lines
        if self.max_class_lines == 0 {
            return Err("max_class_lines must be > 0".to_string());
        }
        if self.max_class_lines > 50000 {
            return Err("max_class_lines unreasonably large (>50000)".to_string());
        }

        // Validate max_complexity
        if self.max_complexity == 0 {
            return Err("max_complexity must be > 0".to_string());
        }
        if self.max_complexity > 100 {
            return Err("max_complexity unreasonably large (>100)".to_string());
        }

        // Validate max_nesting
        if self.max_nesting == 0 {
            return Err("max_nesting must be > 0".to_string());
        }
        if self.max_nesting > 20 {
            return Err("max_nesting unreasonably large (>20)".to_string());
        }

        // Validate max_message_chain
        if self.max_message_chain == 0 {
            return Err("max_message_chain must be > 0".to_string());
        }
        if self.max_message_chain > 20 {
            return Err("max_message_chain unreasonably large (>20)".to_string());
        }

        // Validate max_methods
        if self.max_methods == 0 {
            return Err("max_methods must be > 0".to_string());
        }
        if self.max_methods > 200 {
            return Err("max_methods unreasonably large (>200)".to_string());
        }

        // Validate max_fields
        if self.max_fields == 0 {
            return Err("max_fields must be > 0".to_string());
        }
        if self.max_fields > 100 {
            return Err("max_fields unreasonably large (>100)".to_string());
        }

        // Validate max_return_count
        if self.max_return_count == 0 {
            return Err("max_return_count must be > 0".to_string());
        }
        if self.max_return_count > 20 {
            return Err("max_return_count unreasonably large (>20)".to_string());
        }

        Ok(())
    }
}

// Default value functions for code quality
fn default_max_function_lines() -> usize {
    50
}

fn default_max_parameters() -> usize {
    4
}

fn default_max_class_lines() -> usize {
    300
}

fn default_max_complexity() -> usize {
    10
}

fn default_max_nesting() -> usize {
    4
}

fn default_max_message_chain() -> usize {
    4
}

fn default_max_methods() -> usize {
    20
}

fn default_max_fields() -> usize {
    10
}

fn default_max_return_count() -> usize {
    3
}

// Helper functions for serde defaults
fn default_true() -> bool {
    true
}

fn default_confidence() -> f32 {
    0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [frameworks.react]
            enabled = true
        "#;

        let config: DannyConfig = toml::from_str(toml).unwrap();
        assert!(config.frameworks.contains_key("react"));
        assert!(config.frameworks["react"].enabled);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [frameworks.react]
            enabled = true
            entry_patterns = ["**/*.jsx", "**/*.tsx"]
            component_patterns = ["src/components/**/*.tsx"]
            exclude_patterns = ["**/*.test.tsx"]
            confidence_threshold = 0.8

            [frameworks.nextjs]
            enabled = true
            entry_patterns = ["pages/**/*.tsx", "app/**/*.tsx"]

            [entry_points]
            auto_detect = true
            manual = ["src/index.ts"]
            patterns = ["src/pages/**/*.tsx"]
            exclude = ["**/*.test.ts"]

            [analysis]
            follow_external = false
            max_depth = 10
            workers = 4
        "#;

        let config: DannyConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.frameworks.len(), 2);
        assert_eq!(config.frameworks["react"].confidence_threshold, 0.8);
        assert_eq!(config.entry_points.manual, vec!["src/index.ts"]);
        assert_eq!(config.analysis.max_depth, Some(10));
    }

    #[test]
    fn test_default_values() {
        let toml = r#"
            [frameworks.react]
        "#;

        let config: DannyConfig = toml::from_str(toml).unwrap();
        let react = &config.frameworks["react"];

        assert!(react.enabled); // defaults to true
        assert_eq!(react.confidence_threshold, 0.7); // default
        assert!(react.entry_patterns.is_empty());
    }

    /// Test that default quality configuration passes validation.
    #[test]
    fn test_quality_config_default_validation() {
        let config = CodeQualityConfig::default();
        assert!(config.validate().is_ok());
    }

    /// Test validation catches zero values.
    #[test]
    fn test_quality_config_zero_validation() {
        let mut config = CodeQualityConfig::default();

        config.max_function_lines = 0;
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "max_function_lines must be > 0"
        );

        config = CodeQualityConfig::default();
        config.max_parameters = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_parameters must be > 0");

        config = CodeQualityConfig::default();
        config.max_class_lines = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_class_lines must be > 0");

        config = CodeQualityConfig::default();
        config.max_complexity = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_complexity must be > 0");

        config = CodeQualityConfig::default();
        config.max_nesting = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_nesting must be > 0");

        config = CodeQualityConfig::default();
        config.max_message_chain = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_message_chain must be > 0");

        config = CodeQualityConfig::default();
        config.max_methods = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_methods must be > 0");

        config = CodeQualityConfig::default();
        config.max_fields = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_fields must be > 0");

        config = CodeQualityConfig::default();
        config.max_return_count = 0;
        assert!(config.validate().is_err());
        assert_eq!(config.validate().unwrap_err(), "max_return_count must be > 0");
    }

    /// Test validation catches unreasonably large values.
    #[test]
    fn test_quality_config_upper_bound_validation() {
        let mut config = CodeQualityConfig::default();

        config.max_function_lines = 10001;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_function_lines unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_parameters = 21;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_parameters unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_class_lines = 50001;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_class_lines unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_complexity = 101;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_complexity unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_nesting = 21;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_nesting unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_message_chain = 21;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_message_chain unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_methods = 201;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_methods unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_fields = 101;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_fields unreasonably large"));

        config = CodeQualityConfig::default();
        config.max_return_count = 21;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_return_count unreasonably large"));
    }

    /// Test that reasonable custom values pass validation.
    #[test]
    fn test_quality_config_custom_valid_values() {
        let config = CodeQualityConfig {
            max_function_lines: 100,
            max_parameters: 6,
            max_class_lines: 500,
            max_complexity: 15,
            max_nesting: 5,
            max_message_chain: 5,
            max_methods: 30,
            max_fields: 15,
            max_return_count: 5,
        };

        assert!(config.validate().is_ok());
    }

    /// Test edge cases at the boundaries.
    #[test]
    fn test_quality_config_boundary_values() {
        // Test exact upper bounds (should pass)
        let config = CodeQualityConfig {
            max_function_lines: 10000,
            max_parameters: 20,
            max_class_lines: 50000,
            max_complexity: 100,
            max_nesting: 20,
            max_message_chain: 20,
            max_methods: 200,
            max_fields: 100,
            max_return_count: 20,
        };
        assert!(config.validate().is_ok());

        // Test minimum valid values (should pass)
        let config = CodeQualityConfig {
            max_function_lines: 1,
            max_parameters: 1,
            max_class_lines: 1,
            max_complexity: 1,
            max_nesting: 1,
            max_message_chain: 1,
            max_methods: 1,
            max_fields: 1,
            max_return_count: 1,
        };
        assert!(config.validate().is_ok());
    }
}
