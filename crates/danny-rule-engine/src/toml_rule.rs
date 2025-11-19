//! TOML rule definitions and types
//!
//! This module defines the structure of rules as they appear in TOML files.

use serde::{Deserialize, Serialize};

/// A complete TOML rule file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlRuleFile {
    /// Framework metadata (optional)
    #[serde(default)]
    pub framework: Option<FrameworkMetadata>,

    /// List of rules
    #[serde(default)]
    pub rules: Vec<TomlRule>,

    /// List of entry point patterns (for file discovery BEFORE analysis)
    #[serde(default)]
    pub entry_points: Vec<EntryPointPattern>,
}

/// Framework metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkMetadata {
    /// Framework name (e.g., "React", "Next.js")
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: Option<String>,

    /// Version of this rule file
    #[serde(default)]
    pub version: Option<String>,

    /// Detection hints (for auto-detection) - deprecated, use detection rules instead
    #[serde(default)]
    pub detect: Vec<String>,

    /// Detection priority (higher = evaluated first, default: 50)
    #[serde(default)]
    pub priority: Option<u32>,

    /// Frameworks that this framework suppresses (e.g., Next.js suppresses React)
    #[serde(default)]
    pub suppresses: Vec<String>,

    /// Detection rules for framework auto-detection
    #[serde(default)]
    pub detection: Vec<DetectionRule>,
}

/// Detection rule for framework auto-detection
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetectionRule {
    /// Detection type
    #[serde(rename = "type")]
    pub rule_type: DetectionType,

    /// Pattern to match (regex for import_pattern/export_pattern, glob for file_path)
    pub pattern: String,

    /// Optional weight for this detection rule (default: 1.0)
    #[serde(default)]
    pub weight: Option<f32>,
}

/// Detection rule type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionType {
    /// Match import statements (e.g., "react", "next/*")
    Import,
    /// Match export names (e.g., "^use[A-Z]" for hooks)
    ExportPattern,
    /// Check package.json dependencies
    PackageDependency,
    /// Match package.json scripts
    PackageScript,
    /// Match file path patterns (glob)
    FilePath,
    /// Match file extensions (e.g., ".vue", ".svelte")
    FileExtension,
}

/// A single TOML rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlRule {
    /// Rule name (for debugging/logging)
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Matching conditions
    #[serde(rename = "match")]
    pub matcher: RuleMatcher,

    /// Action to take when matched
    pub action: RuleActionConfig,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: Option<u32>,
}

/// Rule matching conditions
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuleMatcher {
    /// Must import from this module (supports single string or array for backward compatibility)
    #[serde(default, deserialize_with = "deserialize_import_from")]
    pub import_from: Option<Vec<String>>,

    /// Import source must match this regex pattern
    #[serde(default)]
    pub import_from_pattern: Option<String>,

    /// Must import these specific specifiers (named imports)
    #[serde(default)]
    pub import_specifiers: Option<Vec<String>>,

    /// Must import default export
    #[serde(default)]
    pub import_default: Option<bool>,

    /// Must use namespace import (import * as)
    #[serde(default)]
    pub import_namespace: Option<bool>,

    /// Export name must match this regex pattern
    #[serde(default)]
    pub export_pattern: Option<String>,

    /// Export name must be in this list
    #[serde(default)]
    pub export_name: Option<Vec<String>>,

    /// Export must be of this type
    #[serde(default)]
    pub export_type: Option<ExportType>,

    /// File path must start with one of these
    #[serde(default)]
    pub path_starts_with: Option<Vec<String>>,

    /// File path must end with one of these
    #[serde(default)]
    pub path_ends_with: Option<Vec<String>>,

    /// File path must match this regex pattern
    #[serde(default)]
    pub path_pattern: Option<String>,

    // Negation matchers
    /// Export name must NOT match this regex pattern
    #[serde(default)]
    pub not_export_pattern: Option<String>,

    /// Export name must NOT be in this list
    #[serde(default)]
    pub not_export_name: Option<Vec<String>>,

    /// File path must NOT match this regex pattern
    #[serde(default)]
    pub not_path_pattern: Option<String>,

    /// Must NOT import from these modules
    #[serde(default)]
    pub not_import_from: Option<Vec<String>>,

    /// File content must match this regex pattern (e.g., for JSDoc tags)
    #[serde(default)]
    pub content_pattern: Option<String>,

    /// Export must be used at least this many times
    #[serde(default)]
    pub min_usage_count: Option<usize>,

    /// Export must be used at most this many times
    #[serde(default)]
    pub max_usage_count: Option<usize>,
}

/// Custom deserializer for import_from that accepts both String and Vec<String>
fn deserialize_import_from<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct ImportFromVisitor;

    impl<'de> Visitor<'de> for ImportFromVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or array of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(vec![value.to_string()]))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(vec![value]))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(elem) = seq.next_element()? {
                vec.push(elem);
            }
            Ok(Some(vec))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }
    }

    deserializer.deserialize_any(ImportFromVisitor)
}

/// Action configuration from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleActionConfig {
    /// Mark exports as framework-used
    #[serde(default)]
    pub mark_used: Option<bool>,

    /// Reason for marking as used
    #[serde(default)]
    pub reason: Option<String>,

    /// Skip this file/export entirely
    #[serde(default)]
    pub skip: Option<bool>,

    /// Warn but don't error (new action type)
    #[serde(default)]
    pub warn: Option<bool>,

    /// Warning message (used with warn = true)
    #[serde(default)]
    pub message: Option<String>,

    /// Override severity level
    #[serde(default)]
    pub severity: Option<Severity>,
}

/// Severity level for rule actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Error severity (default)
    Error,
    /// Warning severity
    Warn,
    /// Info severity
    Info,
}

impl RuleActionConfig {
    /// Convert to executable RuleAction
    pub fn to_action(&self) -> RuleAction {
        if self.skip == Some(true) {
            RuleAction::Skip
        } else if self.warn == Some(true) {
            RuleAction::Warn {
                message: self.message.clone().unwrap_or_else(|| {
                    self.reason
                        .clone()
                        .unwrap_or_else(|| "Rule matched".to_string())
                }),
            }
        } else if let Some(severity) = self.severity {
            RuleAction::SetSeverity { level: severity }
        } else if self.mark_used == Some(true) {
            RuleAction::MarkUsed {
                reason: self.reason.clone(),
            }
        } else {
            // Default: mark as used if neither specified
            RuleAction::MarkUsed {
                reason: self.reason.clone(),
            }
        }
    }
}

/// Executable rule action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    /// Mark export as framework-used
    MarkUsed { reason: Option<String> },

    /// Skip this file/export
    Skip,

    /// Warn but don't error
    Warn { message: String },

    /// Override severity level
    SetSeverity { level: Severity },
}

/// Export type filter for matching specific kinds of exports
///
/// # ⚠️ IMPORTANT LIMITATIONS
///
/// This uses **naming heuristics**, NOT actual type checking from the AST.
/// The matching may produce **false positives**:
///
/// - `Function`: Accepts any non-empty export name (may match classes/constants)
/// - `Class`: Matches PascalCase exports (may match React components that are functions)
/// - `Const`: Matches camelCase or UPPER_SNAKE_CASE (may miss some cases)
/// - `Enum`: Matches PascalCase exports (same issue as Class)
///
/// ## Why Heuristics?
///
/// Accurate type detection requires access to Fob's symbol table with async graph operations.
/// This is planned for a future release.
///
/// ## Recommendation
///
/// For precise filtering, use `export_name` or `export_pattern` instead of `export_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportType {
    Function,
    Class,
    Const,
    Let,
    Var,
    Type,
    Interface,
    Enum,
}

impl ExportType {
    /// Check if a Fob export kind matches this type
    ///
    /// Uses heuristics based on export name patterns and metadata.
    /// Note: This is a best-effort approach. For accurate type detection,
    /// we would need access to Fob's symbol table, which requires async
    /// access to the module graph. This can be enhanced in the future.
    pub fn matches_fob_export(&self, export: &fob::graph::Export) -> bool {
        // Check type-only exports first (most reliable)
        if export.is_type_only {
            return matches!(self, ExportType::Type | ExportType::Interface);
        }

        // Use naming conventions as heuristics
        // Note: This is imperfect but better than accepting all exports
        match self {
            ExportType::Type | ExportType::Interface => {
                // Type-only exports are already handled above
                // For non-type-only, we can't reliably detect without symbol info
                false
            }
            ExportType::Function => {
                // Functions typically start with lowercase or are PascalCase
                // But classes also use PascalCase, so this is imperfect
                let name = &export.name;
                // Common patterns: camelCase, PascalCase (but could be class)
                // We'll be lenient and accept most patterns
                !name.is_empty()
            }
            ExportType::Class => {
                // Classes typically use PascalCase
                let name = &export.name;
                if name.is_empty() {
                    return false;
                }
                // Check if first character is uppercase (PascalCase)
                name.chars().next().is_some_and(|c| c.is_uppercase())
            }
            ExportType::Const | ExportType::Let | ExportType::Var => {
                // Constants/variables typically use camelCase or UPPER_SNAKE_CASE
                let name = &export.name;
                if name.is_empty() {
                    return false;
                }
                // Accept if not clearly a class (doesn't start with uppercase)
                // or if it's UPPER_SNAKE_CASE (constants)
                name.chars().next().is_some_and(|c| !c.is_uppercase())
                    || name.contains('_') && name.chars().all(|c| c.is_uppercase() || c == '_')
            }
            ExportType::Enum => {
                // Enums typically use PascalCase
                let name = &export.name;
                if name.is_empty() {
                    return false;
                }
                // Check if first character is uppercase
                name.chars().next().is_some_and(|c| c.is_uppercase())
            }
        }
    }
}

/// Entry point pattern for file discovery BEFORE analysis
///
/// Entry points are discovered using glob patterns and used to seed the dependency graph.
/// This is separate from rules, which match modules AFTER analysis to mark exports.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntryPointPattern {
    /// Pattern name (for debugging/logging)
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Glob patterns to match entry point files
    ///
    /// Examples:
    /// - `["**/app/**/page.{ts,tsx,js,jsx}"]` - Next.js App Router pages
    /// - `["pages/**/*.{ts,tsx,js,jsx}"]` - Next.js Pages Router
    /// - `["app/routes/**/*.{ts,tsx,js,jsx}"]` - Remix routes
    pub patterns: Vec<String>,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let toml = r#"
            [[rules]]
            name = "react-hooks"
            description = "React hook pattern"

            [rules.match]
            import_from = "react"
            export_pattern = "^use[A-Z]\\w+"
            export_type = "function"

            [rules.action]
            mark_used = true
            reason = "React hook"
        "#;

        let file: TomlRuleFile = toml::from_str(toml).unwrap();
        assert_eq!(file.rules.len(), 1);
        assert_eq!(file.rules[0].name, "react-hooks");
        assert_eq!(
            file.rules[0].matcher.import_from,
            Some(vec!["react".to_string()])
        );
    }

    #[test]
    fn test_parse_framework_metadata() {
        let toml = r#"
            [framework]
            name = "React"
            description = "React framework"
            version = "1.0.0"
            detect = ["import:react", "import:react-dom"]

            [[rules]]
            name = "test-rule"

            [rules.match]
            export_name = ["default"]

            [rules.action]
            mark_used = true
        "#;

        let file: TomlRuleFile = toml::from_str(toml).unwrap();
        assert!(file.framework.is_some());
        assert_eq!(file.framework.as_ref().unwrap().name, "React");
        assert_eq!(file.framework.as_ref().unwrap().detect.len(), 2);
    }
}
